use crate::{
    chain::client::{Client, ClientKeyKind},
    prelude::*,
};

pub struct WalletConnectUi {
    wallet_connected: Mutable<bool>,
    client_key_kind: Arc<Mutex<Option<ClientKeyKind>>>,
    error: Mutable<Option<String>>,
    phase: Mutable<Phase>,
}

impl WalletConnectUi {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            wallet_connected: Mutable::new(false),
            client_key_kind: Arc::new(Mutex::new(
                CONFIG
                    .debug
                    .auto_connect
                    .as_ref()
                    .map(|x| x.key_kind.clone()),
            )),
            error: Mutable::new(None),
            phase: Mutable::new(Phase::Init),
        })
    }

    pub fn render(self: &Arc<Self>) -> Dom {
        let state = self;

        static CONTENT: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("display", "flex")
                .style("flex-direction", "column")
                .style("justify-content", "center")
                .style("align-items", "center")
                .style("margin-top", "5rem")
                .style("gap", "2rem")
            }
        });

        html!("div", {
            .child(html!("div", {
                .child(html!("div", {
                    .class(&*CONTENT)
                    .child(html!("div", {
                        .style("padding-top", "5rem")
                        .class([FontSize::H3.class(), FontWeight::Bold.class(), &*TEXT_ALIGN_CENTER])
                        .text("Connect your wallet")
                    }))
                    .child(html!("div", {
                        .child_signal(state.wallet_connected.signal().map(clone!(state => move |connected| {
                            if !connected {
                                Some(state.render_connect())
                            } else {
                                // this will only be shown temporarily
                                None
                            }
                        })))
                    }))
                }))
            }))
        })
    }

    fn render_connect(self: &Arc<Self>) -> Dom {
        let state = self;

        html!("div", {
            .future(state.phase.signal_cloned().for_each(clone!(state => move |phase_value| {
                clone!(state => async move {
                    tracing::info!("phase: {:?}", phase_value);
                    match phase_value {
                        Phase::Init => {
                            if state.client_key_kind.lock().unwrap_throw().is_some() {
                                state.phase.set_neq(Phase::Connecting);
                            }
                        },
                        Phase::Connecting => {
                            let res = Client::connect(
                                state.client_key_kind.lock().unwrap_throw().clone().unwrap_throw(),
                            ).await;

                            match res {
                                Ok(_) => {
                                    state.wallet_connected.set(true);
                                    Route::App.go_to_url();
                                },
                                Err(e) => {
                                    tracing::error!("Error connecting: {:?}", e);

                                    match state.client_key_kind.lock().unwrap_throw().as_ref().unwrap_throw() {
                                        ClientKeyKind::Mnemonic(_) => {
                                            state.error.set(Some("Unable to connect".to_string()));
                                        },
                                        ClientKeyKind::Metamask=> {
                                            state.phase.set(Phase::MetamaskError(e.to_string()));
                                        }
                                    }
                                }
                            }
                        },

                        Phase::MetamaskError(_) => {
                        },
                    }
                })
            })))
            .style("display", "flex")
            .style("justify-content", "center")
            .style("align-items", "center")
            .style("gap", "1rem")
            .child_signal(state.phase.signal_cloned().map(clone!(state => move |phase_value| {
                Some(match phase_value {
                    Phase::Init => {
                        state.render_wallet_select(None)
                    },
                    Phase::Connecting => {
                        html!("div", {
                            .class(FontSize::H2.class())
                            .text("Connecting...")
                        })
                    },
                    Phase::MetamaskError(e) => {
                        state.render_wallet_select(Some(e))
                    }
                })
            })))
        })
    }

    fn render_wallet_select(self: &Arc<Self>, error: Option<String>) -> Dom {
        let state = self;

        static CONTAINER: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("display", "flex")
                .style("flex-direction", "column")
                .style("align-items", "center")
                .style("gap", "1rem")
            }
        });
        static DROPDOWNS: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("display", "flex")
                .style("gap", "1rem")
            }
        });

        #[derive(PartialEq, Clone, Copy, Debug)]
        enum SignerKind {
            Mnemonic,
            Metamask,
            Anvil,
        }

        let signer_kind: Mutable<Option<SignerKind>> = Mutable::new(None);

        let disabled_connect_signal = signer_kind.signal().map(|signer_kind| signer_kind.is_none());

        html!("div", {
            .class(&*CONTAINER)
            .child(html!("div", {
                .class(&*DROPDOWNS)
                .children([
                    Label::new()
                        .with_text("Signer")
                        .render(Dropdown::new()
                            .with_intial_selected(signer_kind.get_cloned())
                            .with_options([
                                ("Metamask".to_string(), SignerKind::Metamask),
                                ("Anvil".to_string(), SignerKind::Anvil),
                                ("Mnemonic".to_string(), SignerKind::Mnemonic),
                            ])
                            .with_on_change(clone!(state, signer_kind => move |signer| {
                                match signer {
                                    SignerKind::Mnemonic => {
                                        *state.client_key_kind.lock().unwrap_throw() = Some(ClientKeyKind::Mnemonic("".to_string()))
                                    },
                                    SignerKind::Metamask => {
                                        *state.client_key_kind.lock().unwrap_throw() = Some(ClientKeyKind::Metamask);
                                    },
                                    SignerKind::Anvil => {
                                        *state.client_key_kind.lock().unwrap_throw() = Some(ClientKeyKind::Mnemonic("test test test test test test test test test test test junk".to_string()));
                                    },
                                }
                                signer_kind.set(Some(*signer));

                            }))
                            .render()
                        ),
                ])
            }))
            .child_signal(signer_kind.signal().map(clone!(state => move |signer_kind| {
                match signer_kind {
                    Some(SignerKind::Mnemonic) => {
                        Some(TextArea::new()
                            .with_placeholder("Mnemonic")
                            .with_on_input(clone!(state => move |mnemonic| {
                                *state.client_key_kind.lock().unwrap_throw() = Some(ClientKeyKind::Mnemonic(mnemonic.unwrap_or_default()));
                            }))
                            .with_mixin(|dom| {
                                dom
                                    .class(FontSize::Lg.class())
                                    .style("max-width", "90%")
                                    .style("width", "40rem")
                                    .style("height", "10rem")
                            })
                            .render()
                        )
                    },
                    Some(SignerKind::Metamask) | Some(SignerKind::Anvil) | None => None,
                }
            })))
            .child(Button::new()
                .with_text("Connect")
                .with_disabled_signal(disabled_connect_signal)
                .with_on_click(clone!(state => move || {
                    state.phase.set(Phase::Connecting);
                }))
                .render()
            )
            .apply_if(error.is_some(), |dom| {
                dom.child(html!("div", {
                    .style("margin-top", "1rem")
                    .class([FontSize::Lg.class(), ColorText::Error.class()])
                    .text(error.as_ref().unwrap_throw())
                }))
            })
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Phase {
    Init,
    Connecting,
    MetamaskError(String),
}