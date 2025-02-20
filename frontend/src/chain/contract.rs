use std::sync::LazyLock;

use alloy_provider::{fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller}, network::EthereumWallet, DynProvider, Identity, Provider, RootProvider};
use alloy_sol_types::SolValue;
use shared::price_feed::PriceFeedData;
use trigger::{ITypes::TriggerInfo, WavsTrigger::NewTrigger};
use anyhow::{Result, Context};

use crate::config::CONFIG;

use super::client::{Client, CLIENT};

// Will panic if called in flows where the client is not connected
// but this is inherently gated via UI state
pub static TRIGGER_CONTRACT: LazyLock<TriggerContract> = LazyLock::new(|| {
    let (_, workflow) = CONFIG.unchecked_workflow();
    let address = match workflow.trigger {
        wavs_types::Trigger::EthContractEvent { address, .. } => address,
        _ => unimplemented!()
    };

    let provider = CLIENT.get_cloned().unwrap().provider();

    TriggerContract{
        instance: trigger::WavsTrigger::new(address, provider.clone()),
        provider
    }
});

// Will panic if called in flows where the client is not connected
// but this is inherently gated via UI state
pub static SUBMIT_CONTRACT: LazyLock<SubmitContract> = LazyLock::new(|| {
    let (_, workflow) = CONFIG.unchecked_workflow();
    let address = match workflow.submit {
        wavs_types::Submit::EthereumContract{ address, .. } => address,
        _ => unimplemented!()
    };

    let provider = CLIENT.get_cloned().unwrap().provider();

    SubmitContract {
        instance: submit::WavsSubmit::new(address, provider.clone()),
        provider
    }
});

pub struct TriggerContract {
    instance: trigger::WavsTrigger::WavsTriggerInstance<(), DynProvider>,
    provider: DynProvider,
}

impl TriggerContract {
    pub async fn add_trigger(&self, trigger: Vec<u8>) -> Result<TriggerInfo> {
        let pending = self.instance 
            .addTrigger(trigger.into())
            .gas(1_000_000)
            .send()
            .await?;

        let tx_hash = pending.watch().await?;

        let receipt = self.provider.get_transaction_receipt(tx_hash).await?.context("Transaction not found")?;

        let event = receipt.inner
            .logs()
            .iter()
            .find_map(|log| log.log_decode::<NewTrigger>().map(|log| log.inner.data).ok())
            .context("Event not found")?;

        let trigger_info = TriggerInfo::abi_decode(&event._0, true)?;

        Ok(trigger_info)
    }
}

pub struct SubmitContract {
    instance: submit::WavsSubmit::WavsSubmitInstance<(), DynProvider>,
    provider: DynProvider,
}

impl SubmitContract {
    pub async fn get_price_feed(&self, trigger_id: u64) -> Result<Option<PriceFeedData>> {
        let data = self.instance
            .getData(trigger_id)
            .call()
            .await?
            .data;

        if data.is_empty() {
            Ok(None)
        } else {
            Ok(serde_json::from_slice(&data)?)
        }
    }
}

mod trigger {
    use alloy_sol_macro::sol;

    sol!(
        #[allow(missing_docs)]
        #[sol(rpc)]
        WavsTrigger,
        "../out/WavsTrigger.sol/SimpleTrigger.json"
    );
}

mod submit {
    use alloy_sol_macro::sol;

    sol!(
        #[allow(missing_docs)]
        #[sol(rpc)]
        WavsSubmit,
        "../out/WavsSubmit.sol/SimpleSubmit.json"
    );
}