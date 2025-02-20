use crate::prelude::*;

pub struct LandingUi {
}

impl LandingUi {
    pub fn new() -> Self {
        Self {}
    }

    pub fn render(&self) -> Dom {
        static CONTENT: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("display", "flex")
                .style("flex-direction", "column")
                .style("justify-content", "center")
                .style("align-items", "center")
                .style("margin-top", "5rem")
                .style("gap", "1rem")
            }
        });

        html!("div", {
            .child(html!("div", {
                .class([&*CONTENT])
                .child(html!("div", {
                    .class([FontSize::H1.class(), FontWeight::Bold.class()])
                    .text("WAVS Rust Demo")
                }))
                .child(html!("div", {
                    .class([FontSize::Xlg.class()])
                    .text("This is a demo of integrating with WAVS in the browser")
                }))
                .child_signal(CLIENT.signal_cloned().map(|client| {
                    Some(match client {
                        None => {
                            Button::new()
                                .with_text("Start")
                                .with_link(Route::WalletConnect)
                                .with_size(ButtonSize::Lg)
                                .with_color(ButtonColor::Primary)
                                .render()
                        }
                        Some(_) => {
                            Button::new()
                                .with_text("Start")
                                .with_link(Route::App)
                                .with_size(ButtonSize::Lg)
                                .with_color(ButtonColor::Primary)
                                .render()
                        } 
                    })
                }))
            }))
        })
    }
}