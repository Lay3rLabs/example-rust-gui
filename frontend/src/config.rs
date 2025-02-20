#![allow(dead_code)]
use std::{collections::{BTreeMap, HashMap}, sync::{Arc, LazyLock, Mutex}};

use crate::{chain::client::ClientKeyKind, route::Route};
use anyhow::{Result, Context};
use dominator::clone;
use futures_signals::signal::Mutable;
use serde::Deserialize;
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen_futures::spawn_local;
use wavs_types::{ChainName, Service, ServiceID, Workflow, WorkflowID};

#[derive(Debug)]
pub struct Config {
    pub root_path: &'static str,
    pub chain_name: ChainName,
    pub debug: ConfigDebug,
    pub data: Mutable<Option<Arc<ConfigData>>>,
}

impl Config {
    pub fn unchecked_data(&self) -> Arc<ConfigData> {
        CONFIG.data.get_cloned().unwrap()
    }
    pub fn unchecked_chain_config(&self) -> EthereumChainConfig {
        self.unchecked_data().cli.chains.get_eth_chain(&self.chain_name).unwrap().clone()
    }
    pub fn unchecked_service(&self) -> (ServiceID, Service) {
        self.unchecked_data().deployments.services.first_key_value().map(|(x, y)| (x.clone(), y.clone())).unwrap()
    }

    pub fn unchecked_workflow(&self) -> (WorkflowID, Workflow) {
        self.unchecked_service().1.workflows.first_key_value().map(|(x, y)| (x.clone(), y.clone())).unwrap()
    }
}

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    let config = Config {
        root_path: "",
        chain_name: ChainName::new("local").unwrap(),
        debug: if cfg!(debug_assertions) {
            ConfigDebug::release_mode()
            //ConfigDebug::dev_mode(true)
        } else {
            ConfigDebug::release_mode()
        },
        data: Mutable::new(None)
    };

    spawn_local({
        let data = config.data.clone();
        async move {
            let baseurl = web_sys::window().unwrap().origin();
            let deployments:ConfigDeployments = reqwest::get(format!("{}/config/deployments.json", baseurl)).await.unwrap_throw().json().await.unwrap_throw();
            let cli = reqwest::get(format!("{}/config/cli.toml", baseurl)).await.unwrap_throw().text().await.unwrap_throw();
            let cli = toml::from_str(&cli).unwrap_throw();

            data.set(Some(Arc::new(ConfigData{deployments, cli})));
        }
    });

    config
});

#[derive(Debug)]
pub struct ConfigDebug {
    pub auto_connect: Option<ConfigDebugAutoConnect>,
    pub start_route: Mutex<Option<Route>>,
}

impl ConfigDebug {
    fn dev_mode(autoconnect: bool) -> Self {
        Self {
            auto_connect: if autoconnect {
                Some(ConfigDebugAutoConnect{
                    key_kind: ClientKeyKind::Metamask
                })
                // Some(ConfigDebugAutoConnect{
                //     key_kind: ClientKeyKind::Mnemonic("test test test test test test test test test test test junk".to_string())
                // })
            } else {
                None
            },
            start_route: Mutex::new(Some(Route::App)),
        }
    }

    fn release_mode() -> Self {
        Self {
            auto_connect: None,
            start_route: Mutex::new(None),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigDebugAutoConnect {
    pub key_kind: ClientKeyKind,
}

#[derive(Deserialize, Debug)]
pub struct ConfigData {
    pub deployments: ConfigDeployments,
    pub cli: CliConfig,
}

#[derive(Deserialize, Debug)]
pub struct ConfigDeployments {
    pub services: BTreeMap<ServiceID, Service>    
}

#[derive(Deserialize, Debug)]
pub struct CliConfig {
    pub chains: ChainConfigs
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ChainConfigs {
    /// Cosmos-style chains (including Layer-SDK)
    pub cosmos: BTreeMap<ChainName, CosmosChainConfig>,
    /// Ethereum-style chains
    pub eth: BTreeMap<ChainName, EthereumChainConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CosmosChainConfig {
    pub chain_id: String,
    pub bech32_prefix: String,
    pub rpc_endpoint: Option<String>,
    pub grpc_endpoint: Option<String>,
    pub gas_price: f32,
    pub gas_denom: String,
    pub faucet_endpoint: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EthereumChainConfig {
    pub chain_id: String,
    pub ws_endpoint: Option<String>,
    pub http_endpoint: Option<String>,
    pub aggregator_endpoint: Option<String>,
    pub faucet_endpoint: Option<String>,
}

impl ChainConfigs {
    pub fn get_eth_chain(&self, chain_name: &ChainName) -> Result<EthereumChainConfig> {
        match (self.eth.get(chain_name), self.cosmos.get(chain_name)) {
            (Some(_), Some(_)) => {
                Err(anyhow::anyhow!("Chain {} is both ethereum and cosmos", (chain_name.clone())))
            }
            (Some(eth), None) => Ok(eth.clone()),
            (None, Some(cosmos)) => Err(anyhow::anyhow!("Chain {} is cosmos, expected ethereum", (chain_name.clone()))), 
            (None, None) => Err(anyhow::anyhow!("Chain {} does not exist", (chain_name.clone()))),
        }
    }

    pub fn all_chain_names(&self) -> Vec<ChainName> {
        self.eth.keys().chain(self.cosmos.keys()).cloned().collect()
    }
}