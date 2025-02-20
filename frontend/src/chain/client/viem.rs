use std::{cell::RefCell, rc::Rc, sync::Arc};

use alloy_consensus::{SignableTransaction, TxEip1559, TypedTransaction};
use alloy_json_rpc::RpcError;
use alloy_primitives::{Address, B256};
use alloy_provider::{network::{Ethereum, EthereumWallet, NetworkWallet, TxSigner}, DynProvider, Network, PendingTransactionBuilder, Provider, ProviderBuilder, RootProvider, SendableTx};
use alloy_signer::{sign_transaction_with_chain_id, Signature, Signer};
use alloy_transport::{impl_future, TransportResult};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use futures::channel::oneshot;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use wavs_types::ChainName;

use crate::{chain::client::bindings::METAMASK, config::{EthereumChainConfig, CONFIG}};

use super::bindings::{JsWalletClient, VIEM};

thread_local! {
    static WALLET_CLIENT: Rc<RefCell<Option<JsWalletClient>>> = Rc::new(RefCell::new(None));
}

#[derive(Clone)]
pub struct ViemEthSigningClient {
    pub provider: DynProvider, 
}

impl ViemEthSigningClient {
    pub async fn connect() -> Result<Self> {
        let accounts = METAMASK.request_accounts().await?;
        let account = *accounts.get(0).context("No accounts")?;

        let wallet_client = JsWalletClient::new(account).await?;

        WALLET_CLIENT.with(move |c| {
            *c.borrow_mut() = Some(wallet_client);
        });

        let provider = ViemProvider::new(accounts).await?;
        tracing::info!("Account balance: {:?}", provider.get_balance(account).await?);

        Ok(Self {
            provider: DynProvider::new(provider)
        })
    }
}

#[derive(Clone, Debug)]
pub struct ViemProvider {
    accounts: Vec<Address>,
    inner: DynProvider,
}

impl ViemProvider {
    async fn new(accounts: Vec<Address>) -> Result<Self> {
        let endpoint = CONFIG.unchecked_chain_config().http_endpoint.unwrap();
        let provider = ProviderBuilder::new().on_http(endpoint.parse()?);

        Ok(Self {
            accounts,
            inner: DynProvider::new(provider)
        })
    }

    fn sender(&self) -> Result<Address> {
        self.accounts.get(0).cloned().context("No accounts")
    } 
}


#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for ViemProvider {
    fn root(&self) ->  &RootProvider<Ethereum>  {
        self.inner.root()
    }

    #[cfg(target_arch = "wasm32")]
    async fn send_transaction_internal(
        &self,
        tx: SendableTx<Ethereum>,
    ) -> TransportResult<PendingTransactionBuilder<Ethereum>> {

        match tx {
            SendableTx::Builder(mut tx) => {
                let sender = self.sender().map_err(|e| RpcError::LocalUsageError(e.into()))?;
                let value = WALLET_CLIENT.with(move |client| {
                    let client = client.borrow().as_ref().unwrap().clone();
                    async move {
                        let tx = tx_into_js_value(sender, client.clone(), tx).map_err(|e| RpcError::LocalUsageError(anyhow!("{e:?}").into()))?;
                        web_sys::console::log_1(&tx);
                        client.send_transaction(&tx).await.map_err(|e| RpcError::LocalUsageError(anyhow!("{e:?}").into()))
                    }
                })
                .await?;

                let tx_hash = value.as_string().ok_or(RpcError::LocalUsageError(anyhow!("Could not get Tx hash").into()))?;
                let tx_hash = const_hex::decode(tx_hash).map_err(|e| RpcError::LocalUsageError(anyhow!("{e:?}").into()))?;
                let tx_hash = B256::from_slice(&tx_hash);

                Ok(PendingTransactionBuilder::new(self.root().clone(), tx_hash))
            }
            SendableTx::Envelope(tx) => {
                Err(RpcError::UnsupportedFeature("ViemProvider does not support SendableTx::Envelope"))
                // let encoded_tx = tx.encoded_2718();
                // self.send_raw_transaction(&encoded_tx).await
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn send_transaction_internal(
        &self,
        tx: SendableTx<Ethereum>,
    ) -> TransportResult<PendingTransactionBuilder<Ethereum>> {
        Err(RpcError::UnsupportedFeature("ViemProvider is only for browsers"))
    }
}

fn tx_into_js_value(sender: Address, client: JsWalletClient, tx: <Ethereum as Network>::TransactionRequest) -> anyhow::Result<JsValue> {
    let to = tx.to.and_then(|to| to.to().cloned()).context("No to address")?;

    web_sys::console::log_1(&serde_wasm_bindgen::to_value(&tx).map_err(|e| anyhow!("{e:?}"))?);

    // ideally we want to fill all these fields: https://viem.sh/docs/actions/wallet/sendTransaction
    let js_obj = js_sys::Object::new();

    js_sys::Reflect::set(&js_obj, &JsValue::from_str("account"), &JsValue::from_str(&sender.to_string()));
    js_sys::Reflect::set(&js_obj, &JsValue::from_str("to"), &JsValue::from_str(&to.to_string()));

    if let Some(data) = tx.input.into_input() {
        js_sys::Reflect::set(&js_obj, &JsValue::from_str("data"), &JsValue::from_str(&const_hex::encode(data)));
    }

    // Most of these are untested, some might be missing, just guessing
    if let Some(access_list) = tx.access_list {
        let access_list = serde_wasm_bindgen::to_value(&access_list).map_err(|e| anyhow!("{e:?}"))?;
        js_sys::Reflect::set(&js_obj, &JsValue::from_str("accessList"), &access_list);
    }

    if let Some(auth_list) = tx.authorization_list {
        let auth_list = serde_wasm_bindgen::to_value(&auth_list).map_err(|e| anyhow!("{e:?}"))?;
        js_sys::Reflect::set(&js_obj, &JsValue::from_str("authorizationList"), &auth_list);
    }


    if let Some(gas) = tx.max_fee_per_gas {
        js_sys::Reflect::set(&js_obj, &JsValue::from_str("maxFeePerGas"), &VIEM.parse_ether(&gas.to_string()));
    }

    if let Some(gas) = tx.max_priority_fee_per_gas {
        js_sys::Reflect::set(&js_obj, &JsValue::from_str("maxPriorityFeePerGas"), &VIEM.parse_ether(&gas.to_string()));
    }

    if let Some(value) = tx.value {
        js_sys::Reflect::set(&js_obj, &JsValue::from_str("value"), &VIEM.parse_ether(&value.to_string()));
    }

    Ok(js_obj.into())

}


impl EthereumChainConfig {
    pub fn into_viem(self) -> JsValue {
        // see https://github.com/wevm/viem/blob/main/src/chains/index.ts for all supported chains
        // anvil: https://github.com/wevm/viem/blob/main/src/chains/definitions/anvil.ts
        #[derive(Serialize, Deserialize, Debug)]
        struct ViemChainConfig {
            id: u32,
            name: String,
            #[serde(rename = "nativeCurrency")]
            native_currency: NativeCurrency,
            #[serde(rename = "rpcUrls")]
            rpc_urls: RpcUrls,
        }

        #[derive(Serialize, Deserialize, Debug)]
        struct NativeCurrency {
            decimals: u32,
            name: String,
            symbol: String,
        }

        #[derive(Serialize, Deserialize, Debug)]
        struct RpcUrls {
            default: RpcUrl,
        }

        #[derive(Serialize, Deserialize, Debug)]
        struct RpcUrl {
            http: Vec<String>,
            #[serde(rename = "webSocket")]
            web_socket: Vec<String>,
        }

        serde_wasm_bindgen::to_value(&ViemChainConfig {
            id: self.chain_id.parse().unwrap(),
            name: self.chain_id.to_string(),
            native_currency: NativeCurrency {
                decimals: 18,
                name: "Ether".to_string(),
                symbol: "ETH".to_string(),
            },
            rpc_urls: RpcUrls {
                default: RpcUrl {
                    http: vec![self.http_endpoint.unwrap()],
                    web_socket: vec![self.ws_endpoint.unwrap()],
                },
            },
        }).unwrap()
    }
}