mod local;
mod viem;
mod bindings;

use std::sync::LazyLock;

use alloy_provider::{DynProvider, RootProvider};
use local::{EthClientConfig, LocalEthSigningClient};
use futures_signals::signal::Mutable;
use anyhow::Result;
use viem::ViemEthSigningClient;
use wavs_types::ChainName;

use crate::config::CONFIG;

pub static CLIENT: LazyLock<Mutable<Option<Client>>> = LazyLock::new(|| {
    Mutable::new(None)
});

#[derive(Clone)]
pub enum Client {
    Local(LocalEthSigningClient),
    Viem(ViemEthSigningClient)
}

impl Client {
    // This sets the client in the global static CLIENT var so that it's accessible from anywhere
    pub async fn connect(key_kind: ClientKeyKind) -> Result<()> {
        match key_kind {
            ClientKeyKind::Mnemonic(mnemonic) => {
                let client = LocalEthSigningClient::new(EthClientConfig {
                    ws_endpoint: None,
                    http_endpoint: Some(CONFIG.unchecked_chain_config().http_endpoint.unwrap()),
                    hd_index: None,
                    transport: None,
                }, mnemonic).await?;

                tracing::info!("connected to {} with wallet {}", client.config.http_endpoint.as_ref().unwrap(), client.address());

                CLIENT.set(Some(Client::Local(client)));
                Ok(())
            },
            ClientKeyKind::Metamask => {
                let client = ViemEthSigningClient::connect().await?;
                CLIENT.set(Some(Client::Viem(client)));
                Ok(())
            },
        }
    }

    pub fn provider(&self) -> DynProvider {
        match self {
            Client::Local(client) => client.provider.clone(),
            Client::Viem(client) => client.provider.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ClientKeyKind {
    Mnemonic(String),
    Metamask,
}
