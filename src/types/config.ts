export interface ScheduledWarmupConfig {
    enabled: boolean;
    monitored_models: string[];
}

export interface QuotaProtectionConfig {
    enabled: boolean;
    threshold_percentage: number; // 1-99
    monitored_models: string[];
}

export interface PinnedQuotaModelsConfig {
    models: string[];
}

export interface AppConfig {
    language: string;
    theme: string;
    auto_refresh: boolean;
    refresh_interval: number;
    auto_sync: boolean;
    sync_interval: number;
    default_export_path?: string;
    antigravity_executable?: string;
    antigravity_args?: string[];
    auto_launch?: boolean;
    auto_check_update?: boolean;
    update_check_interval?: number;
    accounts_page_size?: number;
    hidden_menu_items?: string[];
    scheduled_warmup: ScheduledWarmupConfig;
    quota_protection: QuotaProtectionConfig;
    pinned_quota_models: PinnedQuotaModelsConfig;
}
