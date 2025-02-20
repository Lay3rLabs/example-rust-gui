use alloy_primitives::Address;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use anyhow::{anyhow, Result};

use crate::config::CONFIG;


#[wasm_bindgen(js_namespace = window)]
extern "C" {
    #[wasm_bindgen(js_name = "viem")]
    pub type Viem;
    #[wasm_bindgen(js_name = "viem")]
    pub static VIEM: Viem;

    #[wasm_bindgen(method, js_name = "createPublicClient")]
    pub fn create_public_client(this: &Viem, params: &JsValue) -> EthPublicClient;

    #[wasm_bindgen(method, js_name = "createWalletClient")]
    pub fn create_wallet_client(this: &Viem, params: &JsValue) -> JsWalletClientBase;

    #[wasm_bindgen(method, js_name = "custom")]
    pub fn custom_transport(this: &Viem, params: &JsValue) -> JsValue;

    #[wasm_bindgen(method, js_name = "http")]
    pub fn http(this: &Viem) -> JsValue;

    #[wasm_bindgen(method, js_name = "defineChain")]
    pub fn define_chain(this: &Viem, chain_obj: &JsValue) -> JsValue;

    #[wasm_bindgen(method, getter, js_name = "publicActions")]
    pub fn public_actions(this: &Viem) -> JsValue;

    #[wasm_bindgen(method, js_name = "parseEther")]
    pub fn parse_ether(this: &Viem, amount: &str) -> JsValue;
}

#[wasm_bindgen(js_namespace = window)]
extern "C" {
    #[wasm_bindgen(js_name = "ethereum")]
    pub type Metamask;
    #[wasm_bindgen(js_name = "ethereum")]
    pub static METAMASK: Metamask;

    #[wasm_bindgen(method, catch, js_name = "request")]
    pub async fn _request(this: &Metamask, obj: &JsValue) -> Result<JsValue, JsValue>;
}

impl Metamask {
    pub async fn request(&self, method: &str) -> Result<JsValue> {
        #[derive(Serialize, Deserialize, Debug)]
        struct RequestParams {
            method: String
        }

        let obj = serde_wasm_bindgen::to_value(&RequestParams {
            method: method.to_string()
        }).map_err(|e| anyhow!("{e:?}"))?;

        self._request(&obj).await.map_err(|e| anyhow!("{e:?}"))
    }

    pub async fn request_accounts(&self) -> Result<Vec<alloy_primitives::Address>> {
        let accounts:js_sys::Array = METAMASK.request("eth_requestAccounts").await?.unchecked_into();
        let accounts = serde_wasm_bindgen::from_value(accounts.into()).map_err(|e| anyhow!("{e:?}"))?;
        Ok(accounts)
    }
}

#[wasm_bindgen]
extern "C" {
    #[derive(Debug, Clone)]
    pub type EthPublicClient;

    #[wasm_bindgen(method, catch, js_name = "getBlockNumber")]
    pub async fn get_block_number(this: &EthPublicClient) -> Result<JsValue, JsValue>;
}

#[wasm_bindgen]
extern "C" {
    #[derive(Debug, Clone)]
    pub type JsWalletClientBase;

    #[wasm_bindgen(method)]
    pub fn extend(this: &JsWalletClientBase, obj: &JsValue) -> JsWalletClient;
}

#[wasm_bindgen]
extern "C" {
    #[derive(Debug, Clone)]
    pub type JsWalletClient;

    #[wasm_bindgen(method, catch, js_name = "requestAddresses")]
    pub async fn request_addresses(this: &JsWalletClient) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(method, catch, js_name = "signMessage")]
    pub async fn sign_message(this: &JsWalletClient, msg: &JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(method, catch, js_name = "signTransaction")]
    pub async fn sign_transaction(this: &JsWalletClient, msg: &JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(method, catch, js_name = "sendTransaction")]
    pub async fn send_transaction(this: &JsWalletClient, msg: &JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(method, catch, js_name = "sign")]
    pub async fn sign(this: &JsWalletClient, msg: &JsValue) -> Result<JsValue, JsValue>;

    // Same as public client
    #[wasm_bindgen(method, catch, js_name = "getBlockNumber")]
    pub async fn get_block_number(this: &JsWalletClient) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(method, catch, js_name = "getBalance")]
    pub async fn _get_balance(this: &JsWalletClient, obj: &JsValue) -> Result<JsValue, JsValue>;
}

impl JsWalletClient {
    pub async fn new(account: Address) -> Result<Self> {
        let viem_config = js_sys::Object::new();
        let chain_config = CONFIG.unchecked_chain_config().into_viem();

        let viem_chain = VIEM.define_chain(&chain_config);

        js_sys::Reflect::set(
            &viem_config,
            &JsValue::from_str("chain"),
            &viem_chain,
        )
        .unwrap();

        js_sys::Reflect::set(
            &viem_config,
            &JsValue::from_str("transport"),
            &VIEM.custom_transport(&METAMASK),
        )
        .unwrap();

        // https://viem.sh/docs/actions/wallet/signMessage#account-hoisting

        js_sys::Reflect::set(
            &viem_config,
            &JsValue::from_str("account"),
            &JsValue::from_str(&account.to_string()),
        )
        .unwrap();

        let client = VIEM.create_wallet_client(&viem_config);

        Ok(client.extend(&VIEM.public_actions()))

    }

    pub async fn get_balance(&self, address: alloy_primitives::Address) -> Result<js_sys::BigInt> {
        #[derive(Serialize, Deserialize, Debug)]
        struct RequestParams {
            address: String
        }

        let obj = serde_wasm_bindgen::to_value(&RequestParams {
            address: address.to_string()
        }).map_err(|e| anyhow!("{e:?}"))?;

        let res = self._get_balance(&obj).await.map_err(|e| anyhow!("{e:?}"))?;

        Ok(res.unchecked_into())
    }
}