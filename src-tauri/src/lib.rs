mod models;
mod modules;
mod commands;
mod utils;
pub mod error;
pub mod constants;

use tauri::Manager;
use modules::logger;
use tracing::{info, warn, error};

#[derive(Clone, Copy)]
struct AppRuntimeFlags {
    tray_enabled: bool,
}

fn env_flag_enabled(name: &str) -> bool {
    std::env::var(name)
        .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn is_wayland_session() -> bool {
    std::env::var("WAYLAND_DISPLAY")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
        || std::env::var("XDG_SESSION_TYPE")
            .map(|v| v.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false)
}

fn should_enable_tray() -> bool {
    if env_flag_enabled("ANTIGRAVITY_DISABLE_TRAY") {
        info!("Tray disabled by ANTIGRAVITY_DISABLE_TRAY");
        return false;
    }

    #[cfg(target_os = "linux")]
    {
        if is_wayland_session() && !env_flag_enabled("ANTIGRAVITY_FORCE_TRAY") {
            warn!(
                "Linux Wayland session detected; disabling tray by default to avoid GTK/AppIndicator crashes. Set ANTIGRAVITY_FORCE_TRAY=1 to force-enable."
            );
            return false;
        }
    }

    true
}

#[cfg(target_os = "linux")]
fn configure_linux_gdk_backend() {
    if std::env::var("GDK_BACKEND").is_ok() {
        return;
    }

    let is_wayland = is_wayland_session();
    let has_x11_display = std::env::var("DISPLAY")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    let force_wayland = env_flag_enabled("ANTIGRAVITY_FORCE_WAYLAND");
    let force_x11 = env_flag_enabled("ANTIGRAVITY_FORCE_X11");

    if force_x11 || (is_wayland && has_x11_display && !force_wayland) {
        // Force X11 backend under Wayland sessions to avoid a GTK Wayland shm crash.
        std::env::set_var("GDK_BACKEND", "x11");
        warn!(
            "Forcing GDK_BACKEND=x11 for stability on Wayland. Set ANTIGRAVITY_FORCE_WAYLAND=1 to keep Wayland backend."
        );
    }
}

/// Increase file descriptor limit for macOS to prevent "Too many open files" errors
#[cfg(target_os = "macos")]
fn increase_nofile_limit() {
    unsafe {
        let mut rl = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };

        if libc::getrlimit(libc::RLIMIT_NOFILE, &mut rl) == 0 {
            info!("Current open file limit: soft={}, hard={}", rl.rlim_cur, rl.rlim_max);

            // Attempt to increase to 4096 or maximum hard limit
            let target = 4096.min(rl.rlim_max);
            if rl.rlim_cur < target {
                rl.rlim_cur = target;
                if libc::setrlimit(libc::RLIMIT_NOFILE, &rl) == 0 {
                    info!("Successfully increased hard file limit to {}", target);
                } else {
                    warn!("Failed to increase file descriptor limit");
                }
            }
        }
    }
}

// Test command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Increase file descriptor limit (macOS only)
    #[cfg(target_os = "macos")]
    increase_nofile_limit();

    // Initialize logger
    logger::init_logger();

    #[cfg(target_os = "linux")]
    configure_linux_gdk_backend();

    let tray_enabled = should_enable_tray();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = app.get_webview_window("main")
                .map(|window| {
                    let _ = window.show();
                    let _ = window.set_focus();
                    #[cfg(target_os = "macos")]
                    app.set_activation_policy(tauri::ActivationPolicy::Regular).unwrap_or(());
                });
        }))
        .manage(AppRuntimeFlags { tray_enabled })
        .setup(|app| {
            info!("Setup starting...");

            // Initialize log bridge with app handle for debug console
            modules::log_bridge::init_log_bridge(app.handle().clone());

            // Linux: Workaround for transparent window crash/freeze
            // The transparent window feature is unstable on Linux with WebKitGTK
            // We disable the visual alpha channel to prevent softbuffer-related crashes
            #[cfg(target_os = "linux")]
            {
                use tauri::Manager;
                if is_wayland_session() {
                    info!("Linux Wayland session detected; skipping transparent window workaround");
                } else if let Some(window) = app.get_webview_window("main") {
                    // Access GTK window and disable transparency at the GTK level
                    if let Ok(gtk_window) = window.gtk_window() {
                        use gtk::prelude::WidgetExt;
                        // Remove the visual's alpha channel to disable transparency
                        if let Some(screen) = gtk_window.screen() {
                            // Use non-composited visual if available
                            if let Some(visual) = screen.system_visual() {
                                gtk_window.set_visual(Some(&visual));
                            }
                            info!("Linux: Applied transparent window workaround");
                        }
                    }
                }
            }

            let runtime_flags = app.state::<AppRuntimeFlags>();
            if runtime_flags.tray_enabled {
                modules::tray::create_tray(app.handle())?;
                info!("Tray created");
            } else {
                info!("Tray disabled for this session");
            }

            // 启动智能调度器
            modules::scheduler::start_scheduler(Some(app.handle().clone()));

            info!("Setup completed");

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let tray_enabled = window
                    .app_handle()
                    .try_state::<AppRuntimeFlags>()
                    .map(|flags| flags.tray_enabled)
                    .unwrap_or(true);

                if tray_enabled {
                    let _ = window.hide();
                    #[cfg(target_os = "macos")]
                    {
                        use tauri::Manager;
                        window
                            .app_handle()
                            .set_activation_policy(tauri::ActivationPolicy::Accessory)
                            .unwrap_or(());
                    }
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            // Account management commands
            commands::list_accounts,
            commands::add_account,
            commands::delete_account,
            commands::delete_accounts,
            commands::reorder_accounts,
            commands::switch_account,
            commands::export_accounts,
            // Device fingerprint
            commands::get_device_profiles,
            commands::bind_device_profile,
            commands::bind_device_profile_with_profile,
            commands::preview_generate_profile,
            commands::apply_device_profile,
            commands::restore_original_device,
            commands::list_device_versions,
            commands::restore_device_version,
            commands::delete_device_version,
            commands::open_device_folder,
            commands::get_current_account,
            // Quota commands
            commands::fetch_account_quota,
            commands::refresh_all_quotas,
            // Config commands
            commands::load_config,
            commands::save_config,
            // Additional commands
            commands::prepare_oauth_url,
            commands::start_oauth_login,
            commands::complete_oauth_login,
            commands::cancel_oauth_login,
            commands::submit_oauth_code,
            commands::import_v1_accounts,
            commands::import_from_db,
            commands::import_custom_db,
            commands::sync_account_from_db,
            commands::save_text_file,
            commands::read_text_file,
            commands::clear_log_cache,
            commands::clear_antigravity_cache,
            commands::get_antigravity_cache_paths,
            commands::open_data_folder,
            commands::get_data_dir_path,
            commands::show_main_window,
            commands::set_window_theme,
            commands::get_antigravity_path,
            commands::get_antigravity_args,
            commands::check_for_updates,
            commands::check_homebrew_installation,
            commands::brew_upgrade_cask,
            commands::get_update_settings,
            commands::save_update_settings,
            commands::should_check_updates,
            commands::update_last_check_time,
            commands::toggle_proxy_status,
            // Autostart commands
            commands::autostart::toggle_auto_launch,
            commands::autostart::is_auto_launch_enabled,
            commands::update_account_label,
            // Debug console commands
            modules::log_bridge::enable_debug_console,
            modules::log_bridge::disable_debug_console,
            modules::log_bridge::is_debug_console_enabled,
            modules::log_bridge::get_debug_console_logs,
            modules::log_bridge::clear_debug_console_logs,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            match event {
                // Handle app exit - cleanup background tasks
                tauri::RunEvent::Exit => {
                    tracing::info!("Application exiting, cleaning up background tasks...");
                }
                // Handle macOS dock icon click to reopen window
                #[cfg(target_os = "macos")]
                tauri::RunEvent::Reopen { .. } => {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.unminimize();
                        let _ = window.set_focus();
                        app_handle.set_activation_policy(tauri::ActivationPolicy::Regular).unwrap_or(());
                    }
                }
                _ => {}
            }
        });
}
