use once_cell::sync::Lazy;
use rquest::Client;
use rquest_util::Emulation;

/// 全局共享 HTTP 客户端（15s 超时）
/// Client 内置连接池，clone 是轻量操作
pub static SHARED_CLIENT: Lazy<Client> = Lazy::new(|| create_base_client(15));

/// 全局共享 HTTP 客户端（长超时: 60s）
pub static SHARED_CLIENT_LONG: Lazy<Client> = Lazy::new(|| create_base_client(60));

/// 基础客户端创建逻辑
fn create_base_client(timeout_secs: u64) -> Client {
    let builder = Client::builder()
        .emulation(Emulation::Chrome123)
        .timeout(std::time::Duration::from_secs(timeout_secs));

    // deprecated: 反代功能已移除，不再从配置读取上游代理

    tracing::info!("Initialized JA3/TLS Impersonation (Chrome123)");
    builder.build().unwrap_or_else(|_| Client::new())
}

/// 获取统一配置的 HTTP 客户端（15s 超时）
pub fn get_client() -> Client {
    SHARED_CLIENT.clone()
}

/// 获取长超时 HTTP 客户端（60s 超时）
pub fn get_long_client() -> Client {
    SHARED_CLIENT_LONG.clone()
}
