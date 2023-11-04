use std::time::Duration;

pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_TITLE: &str = "Tundra";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_HOMEPAGE: &str = env!("CARGO_PKG_HOMEPAGE");
pub const APP_HOMEPAGE_NAME: &str = "tundra.moe";
pub const APP_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
pub const APP_COPYRIGHT: &str = "© 2020-2023 Mateusz Maćkowski";

pub const GETTEXT_PACKAGE: &str = "tundra";

pub const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub const MAL_URL: &str = "https://api.myanimelist.net/v2";
pub const MAL_CLIENT_ID: &str = "61c9c7ae268592c2bbe6196c4c1d8aea";
pub const MAL_AUTH_URL: &str = "https://myanimelist.net/v1/oauth2/authorize";
pub const MAL_TOKEN_URL: &str = "https://myanimelist.net/v1/oauth2/token";

// Check players every REFRESH_INTERVAL seconds
pub const REFRESH_INTERVAL: Duration = if cfg!(debug_assertions) {
    Duration::from_secs(1)
} else {
    Duration::from_secs(20)
};
