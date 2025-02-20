use std::sync::Arc;

use alloy_primitives::Address;
use alloy_provider::{fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller}, network::{Ethereum, EthereumWallet}, DynProvider, Identity, ProviderBuilder, RootProvider};
use alloy_signer::k256::ecdsa::SigningKey;
use alloy_signer_local::{coins_bip39::English, LocalSigner, MnemonicBuilder};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::EthereumChainConfig;

#[derive(Clone)]
pub struct LocalEthSigningClient {
    pub config: EthClientConfig,
    pub provider: DynProvider,
    /// The wallet is a collection of signers, with one designated as the default signer
    /// it allows signing transactions
    pub wallet: Arc<EthereumWallet>,
    /// The signer is the same as the default signer in the wallet, but used for simple message signing
    /// due to type system limitations, we need to store it separately
    /// since the signer in `EthereumWallet` implements only `TxSigner`
    /// and there is not a direct way convert it into `Signer`
    pub signer: Arc<LocalSigner<SigningKey>>,
}

impl LocalEthSigningClient {
    pub async fn new(config: EthClientConfig, mnemonic: String) -> Result<LocalEthSigningClient> {
        let signer = MnemonicBuilder::<English>::default()
            .phrase(mnemonic)
            .index(config.hd_index.unwrap_or(0))?
            .build()?;

        let wallet: EthereumWallet = signer.clone().into();

        let endpoint = config.endpoint()?;

        let provider = ProviderBuilder::new()
            .wallet(wallet.clone())
            .on_builtin(&endpoint)
            .await?;

        Ok(LocalEthSigningClient {
            config,
            provider: DynProvider::new(provider),
            wallet: Arc::new(wallet),
            signer: Arc::new(signer),
        })
    }
}

impl EthereumChainConfig {
    pub fn to_client_config(
        &self,
        hd_index: Option<u32>,
        transport: Option<EthClientTransport>,
    ) -> EthClientConfig {
        EthClientConfig {
            ws_endpoint: self.ws_endpoint.clone(),
            http_endpoint: self.http_endpoint.clone(),
            transport,
            hd_index,
        }
    }
}

impl std::fmt::Debug for LocalEthSigningClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalEthSigningClient")
            .field("ws_endpoint", &self.config.ws_endpoint)
            .field("http_endpoint", &self.config.http_endpoint)
            .field("address", &self.address())
            .finish()
    }
}

impl LocalEthSigningClient {
    pub fn address(&self) -> Address {
        self.signer.address()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct EthClientConfig {
    pub ws_endpoint: Option<String>,
    pub http_endpoint: Option<String>,
    pub hd_index: Option<u32>,
    /// Preferred transport
    pub transport: Option<EthClientTransport>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum EthClientTransport {
    WebSocket,
    Http,
}

impl EthClientConfig {
    pub fn endpoint(&self) -> Result<String> {
        let preferred_transport = match (self.transport.as_ref(), self.ws_endpoint.as_ref()) {
            // Http preferred or no preference and no websocket
            (Some(EthClientTransport::Http), _) | (None, None) => EthClientTransport::Http,
            // Otherwise try to connect to websocket
            _ => EthClientTransport::WebSocket,
        };

        match preferred_transport {
            // Http preferred or no preference and no websocket
            EthClientTransport::Http => self.http_endpoint.clone().context("no http endpoint"),
            EthClientTransport::WebSocket => self.ws_endpoint.clone().context("Websocket is preferred transport, but endpoint was not provided")
        }
    }
}