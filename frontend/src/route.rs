use crate::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub enum Route {
    Landing,
    WalletConnect,
    App,
    NotFound,
}

impl Route {
    pub fn from_url(url: &str) -> Self {
        let url = web_sys::Url::new(url).unwrap_throw();
        let paths = url.pathname();
        let paths = paths
            .split('/')
            .into_iter()
            // skip all the roots (1 for the domain, 1 for each part of root path)
            .skip(CONFIG.root_path.chars().filter(|c| *c == '/').count() + 1)
            .collect::<Vec<_>>();
        let paths = paths.as_slice();

        // if we need, we can get query params like:
        //let uid = url.search_params().get("uid");

        match paths {
            [""] => Self::Landing,
            ["wallet-connect"] => Self::WalletConnect,
            ["app"] => Self::App,
            _ => Self::NotFound,
        }
    }

    pub fn link(&self) -> String {
        let s = format!("{}/{}", CONFIG.root_path, self.to_string());
        s.trim_end_matches(r#"//"#).to_string()
    }

    pub fn go_to_url(&self) {
        dominator::routing::go_to_url(&self.link());
    }

    #[allow(dead_code)]
    pub fn hard_redirect(&self) {
        let location = web_sys::window().unwrap_throw().location();
        let s: String = self.link();
        location.set_href(&s).unwrap_throw();
    }

    pub fn signal() -> impl Signal<Item = Route> {
        dominator::routing::url()
            .signal_cloned()
            .map(|url| Route::from_url(&url))
    }
}

impl std::fmt::Display for Route {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: String = match self {
            Route::Landing => "/".to_string(), 
            Route::WalletConnect => "wallet-connect".to_string(), 
            Route::App => "app".to_string(),
            Route::NotFound => "404".to_string(), 
        };
        write!(f, "{}", s)
    }
}