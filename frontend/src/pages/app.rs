use alloy_provider::Provider;
use alloy_sol_types::SolType;
use dominator_helpers::futures::AsyncLoader;
use futures::{channel::mpsc::{self, Receiver}, Stream, StreamExt};
use gloo_timers::future::{IntervalStream, TimeoutFuture};
use shared::price_feed::PriceFeedData;
use wasm_bindgen_futures::spawn_local;
use crate::{chain::contract::{SUBMIT_CONTRACT, TRIGGER_CONTRACT}, prelude::*};

pub struct AppUi { 
    pub error: Mutable<Option<String>>,
    pub trigger_id: Mutable<Option<u64>>,
    pub price_feed: Mutable<Option<Arc<PriceFeedData>>>,
    pub loader: AsyncLoader,
}

impl AppUi {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            error: Mutable::new(None),
            trigger_id: Mutable::new(None),
            price_feed: Mutable::new(None),
            loader: AsyncLoader::new()
        })
    }

    pub fn render(self: &Arc<Self>) -> Dom {
        static CONTAINER: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("margin-top", "2rem")
                .style("display", "flex")
                .style("flex-direction", "column")
                .style("align-items", "center")
                .style("gap", "1rem")
            }
        });

        let state = self;

        html!("div", {
            .class(&*CONTAINER)
            .child(html!("div", {
                .class([FontSize::H1.class(), &*TEXT_ALIGN_CENTER])
                .text("App")
            }))
            .child(Button::new()
                .with_text("Send Transaction")
                .with_on_click(clone!(state => move || {
                    state.loader.load(clone!(state => async move {
                        state.error.set(None);
                        state.trigger_id.set(None);
                        state.price_feed.set(None);

                        match TRIGGER_CONTRACT.add_trigger(b"1".to_vec()).await {
                            Ok(trigger_info) => {
                                state.trigger_id.set(Some(trigger_info.triggerId));
                                state.wait_for_trigger(trigger_info.triggerId).await;
                            },
                            Err(e) => {
                                state.error.set(Some(e.to_string()));
                            },
                        }
                    }))
                }))
                .render()
            )
            .child_signal(state.loader.is_loading().map(|is_loading| {
                if is_loading {
                    Some(html!("div", {
                        .class([FontSize::H3.class(), &*TEXT_ALIGN_CENTER])
                        .text("Loading...")
                    }))
                } else {
                    None
                }
            }))
            .child_signal(state.trigger_id.signal_cloned().map(clone!(state => move |trigger_id| {
                trigger_id.map(|trigger_id| {
                    html!("div", {
                        .class([FontSize::H3.class(), &*TEXT_ALIGN_CENTER])
                        .text(&format!("Trigger ID: {}", trigger_id))
                    })
                })
            })))
            .child_signal(state.price_feed.signal_cloned().map(clone!(state => move |price_feed| {
                price_feed.map(|price_feed| {
                    static CONTAINER: LazyLock<String> = LazyLock::new(|| {
                        class! {
                            .style("margin-top", "2rem")
                            .style("display", "flex")
                            .style("flex-direction", "column")
                            .style("align-items", "center")
                            .style("gap", "1rem")
                        }
                    });
                    html!("div", {
                        .class([&*CONTAINER, FontSize::H3.class()])
                        .children([
                            html!("div", {
                                .text(&format!("Symbol: {}", price_feed.symbol))
                            }),
                            html!("div", {
                                .text(&format!("Timestamp: {}", price_feed.timestamp))
                            }),
                            html!("div", {
                                .text(&format!("Price: {}", price_feed.price))
                            }),
                        ])
                    })
                })
            })))
            .child_signal(state.error.signal_cloned().map(clone!(state => move |error| {
                error.map(|error| {
                    html!("div", {
                        .class([FontSize::H3.class(), ColorText::Error.class(), &*TEXT_ALIGN_CENTER])
                        .text(&error)
                    })
                })
            })))


            
        })
    }

    async fn wait_for_trigger(self: &Arc<Self>, trigger_id: u64) {
        let state = self;
        let performance = web_sys::window().unwrap().performance().unwrap();
        let timeout = performance.now() + 10_000.0;
        loop {
            if performance.now() > timeout {
                state.error.set(Some("Timeout!".to_string()));
                break;
            }

            match SUBMIT_CONTRACT.get_price_feed(trigger_id).await {
                Ok(Some(price_feed)) => {
                    state.price_feed.set(Some(Arc::new(price_feed)));
                    break;
                },
                Err(e) => {
                    state.error.set(Some(e.to_string()));
                    break;
                },
                Ok(None) => {
                    // still waiting...
                }
            }
        }
    }
}