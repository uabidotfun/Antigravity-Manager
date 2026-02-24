use tokio::time::{self, Duration};
use crate::modules::{config, logger, account};

/// 启动周期性配额刷新调度器（反代预热功能已移除）
pub fn start_scheduler(app_handle: Option<tauri::AppHandle>) {
    tauri::async_runtime::spawn(async move {
        logger::log_info("Scheduler started. Periodic quota refresh enabled.");

        // 每 10 分钟扫描一次
        let mut interval = time::interval(Duration::from_secs(600));

        loop {
            interval.tick().await;

            // 加载配置
            let Ok(app_config) = config::load_app_config() else {
                continue;
            };

            if !app_config.auto_refresh {
                continue;
            }

            // 获取所有账号
            let Ok(accounts) = account::list_accounts() else {
                continue;
            };

            if accounts.is_empty() {
                continue;
            }

            logger::log_info(&format!(
                "[Scheduler] 开始周期性配额刷新，共 {} 个账号...",
                accounts.len()
            ));

            // 执行批量刷新
            match account::refresh_all_quotas_logic().await {
                Ok(stats) => {
                    logger::log_info(&format!(
                        "[Scheduler] 配额刷新完成: {} 成功, {} 失败",
                        stats.success, stats.failed
                    ));
                }
                Err(e) => {
                    logger::log_error(&format!(
                        "[Scheduler] 配额刷新失败: {}", e
                    ));
                }
            }

            // 同步到前端
            if let Some(handle) = app_handle.as_ref() {
                let handle_inner = handle.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    let _ = crate::commands::refresh_all_quotas_internal(Some(handle_inner)).await;
                    logger::log_info("[Scheduler] 配额数据已同步到前端");
                });
            }
        }
    });
}
