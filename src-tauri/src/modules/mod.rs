pub mod account;
pub mod quota;
pub mod config;
pub mod logger;
pub mod db;
pub mod process;
pub mod oauth;
pub mod oauth_server;
pub mod migration;
pub mod tray;
pub mod i18n;
pub mod device;
pub mod update_checker;
pub mod scheduler;
pub mod integration;
pub mod account_service;
pub mod cache;
pub mod log_bridge;
pub mod version;

use crate::models;

// Re-export commonly used functions to the top level of the modules namespace for easy external calling
pub use account::*;
#[allow(unused_imports)]
pub use quota::*;
pub use config::*;
#[allow(unused_imports)]
pub use logger::*;
// pub use device::*;

pub async fn fetch_quota(access_token: &str, email: &str, account_id: Option<&str>) -> crate::error::AppResult<(models::QuotaData, Option<String>)> {
    quota::fetch_quota(access_token, email, account_id).await
}
