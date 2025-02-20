#![allow(warnings)]
mod theme;
mod atoms;
mod util;
mod prelude;
mod pages;
mod route;
mod logger;
mod config;
mod chain;
mod header;

use header::Header;
use pages::{app::AppUi, landing::LandingUi, not_found::NotFoundUi, wallet_connect::WalletConnectUi};
use prelude::*;

pub fn main() {
    wasm_bindgen_futures::spawn_local(async {
        init().await;
    });
}

async fn init() {
    logger::init_logger();
    theme::stylesheet::init();

    let sig = || map_ref! {
        let route = Route::signal(),
        let client = CLIENT.signal_ref(|client| client.is_some()),
        let data_ready = CONFIG.data.signal_ref(|data| data.is_some()),
        => {
            if !data_ready {
                None
            } else {
                Some(match route {
                    Route::Landing 
                    | Route::WalletConnect
                    | Route::NotFound => route.clone(),
                    Route::App => if *client {
                        route.clone()
                    } else {
                        Route::WalletConnect
                    }
                })
            }
        }
    };
    dominator::append_dom(&dominator::body(), 
        html!("div", {
            .future(CLIENT.signal_cloned().for_each(|client| {
                async move {
                    // for debugging, we want to jump to an initial page, but:
                    // 1. only consider it after connection status has settled
                    // 2. only one time (not if we intentionally come back to landing)
                    if client.is_some() {
                        let start_route = CONFIG.debug.start_route.lock().unwrap_throw().take();
                        if let Some(start_route) = start_route {
                            start_route.go_to_url();
                        }
                    }
                }
            }))
            .child_signal(sig().map(|route| {
                route.and_then(|route| {
                    match route {
                        Route::Landing => None,
                        _ => Some(Header::new().render())
                    }
                })
            }))
            .child_signal(sig().map(|route| {
                route.map(|route| {
                    match route {
                        Route::Landing => LandingUi::new().render(),
                        Route::WalletConnect => WalletConnectUi::new().render(),
                        Route::App => AppUi::new().render(),
                        Route::NotFound => NotFoundUi::new().render()
                    }
                })
            }))
            .fragment(&Modal::render())
        })
    );
}