use once_cell::sync::Lazy;
use rquest::Client;
use rquest::tls::CertStore;
use rquest_util::Emulation;

/// 全局共享 HTTP 客户端（15s 超时）
/// Client 内置连接池，clone 是轻量操作
pub static SHARED_CLIENT: Lazy<Client> = Lazy::new(|| create_base_client(15));

/// 全局共享 HTTP 客户端（长超时: 60s）
pub static SHARED_CLIENT_LONG: Lazy<Client> = Lazy::new(|| create_base_client(60));

/// 基础客户端创建逻辑
fn create_base_client(timeout_secs: u64) -> Client {
    let mut builder = Client::builder()
        .emulation(Emulation::Chrome123)
        .timeout(std::time::Duration::from_secs(timeout_secs));

    // 加载系统原生 CA 证书库，使 MitM 代理（Surge/Charles/Clash 等）
    // 的 CA 证书在系统信任后能被应用识别
    match load_native_cert_store() {
        Some(store) => {
            builder = builder.cert_store(store);
            tracing::info!("已加载系统原生 CA 证书库");
        }
        None => {
            tracing::warn!("无法加载系统原生 CA 证书，将仅使用内置 webpki 根证书");
        }
    }

    tracing::info!("Initialized JA3/TLS Impersonation (Chrome123)");
    builder.build().unwrap_or_else(|_| Client::new())
}

/// 从操作系统信任存储加载原生 CA 证书，构建 rquest 可用的 CertStore。
/// 支持 macOS Keychain、Windows 证书存储、Linux 系统 CA 目录。
/// 用户在系统中信任的 MitM 代理 CA 证书将自动包含在内。
fn load_native_cert_store() -> Option<CertStore> {
    // 使用 rustls-native-certs 加载操作系统信任存储中的所有 CA 证书
    // load_native_certs() 返回 CertificateResult { certs, errors }
    let result = rustls_native_certs::load_native_certs();

    // 记录加载过程中的错误（非致命，部分证书可能解析失败）
    for err in &result.errors {
        tracing::warn!("加载系统证书时遇到错误: {:?}", err);
    }

    if result.certs.is_empty() {
        tracing::warn!("系统证书库为空，跳过原生 CA 加载");
        return None;
    }

    tracing::info!("发现 {} 个系统原生 CA 证书", result.certs.len());

    // 将 DER 编码的系统证书添加到 CertStore 中
    // CertificateInput 接受 &[u8]（DER 格式），CertificateDer 实现了 AsRef<[u8]>
    let store = CertStore::builder()
        .add_der_certs(result.certs.iter().map(|c| c.as_ref()))
        .build();

    match store {
        Ok(s) => Some(s),
        Err(e) => {
            tracing::warn!("构建证书存储失败: {}", e);
            None
        }
    }
}

/// 获取统一配置的 HTTP 客户端（15s 超时）
pub fn get_client() -> Client {
    SHARED_CLIENT.clone()
}

/// 获取长超时 HTTP 客户端（60s 超时）
pub fn get_long_client() -> Client {
    SHARED_CLIENT_LONG.clone()
}
