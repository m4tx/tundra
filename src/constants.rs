use std::time::Duration;

pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

pub const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
pub const MAL_CLIENT_ID: &str = "6114d00ca681b7701d1e15fe11a4987e";

// Check players every REFRESH_INTERVAL seconds
pub const REFRESH_INTERVAL: Duration = Duration::from_secs(20);
