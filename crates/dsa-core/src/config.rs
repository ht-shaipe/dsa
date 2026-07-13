//! DSA 应用配置

use serde::{Deserialize, Serialize};
use std::path::Path;

/// 应用全局配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub stock: StockConfig,
    pub llm: LlmConfig,
    pub database: DatabaseConfig,
    pub scheduler: SchedulerConfig,
    pub market_review: MarketReviewConfig,
    pub backtest: BacktestConfig,
    pub agent: AgentConfig,
    #[serde(default)]
    pub notification: NotificationConfig,
    #[serde(default)]
    pub search: SearchConfig,
    #[serde(default)]
    pub proxy: ProxyConfig,
    #[serde(default)]
    pub social_sentiment: SocialSentimentConfig,
    #[serde(default)]
    pub news: NewsConfig,
    #[serde(default)]
    pub report: ReportConfig,
    #[serde(default)]
    pub bot: BotConfig,
    #[serde(default)]
    pub portfolio_risk: PortfolioRiskConfig,
    #[serde(default)]
    pub data_source: DataSourceConfig,
}

/// 通知渠道配置（钉钉/飞书/企微/Telegram/邮件等）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    #[serde(default)]
    pub dingtalk_webhook: String,
    #[serde(default)]
    pub feishu_webhook: String,
    #[serde(default)]
    pub wecom_webhook: String,
    #[serde(default)]
    pub telegram_bot_token: String,
    #[serde(default)]
    pub telegram_chat_id: String,
    #[serde(default)]
    pub bark_url: String,
    #[serde(default)]
    pub email_smtp_host: String,
    #[serde(default)]
    pub email_smtp_port: u16,
    #[serde(default)]
    pub email_user: String,
    #[serde(default)]
    pub email_pass: String,
    #[serde(default)]
    pub email_pass_env: String,
    #[serde(default)]
    pub email_from: String,
    #[serde(default)]
    pub email_to: String,
    #[serde(default)]
    pub discord_webhook: String,
    #[serde(default)]
    pub slack_webhook: String,
    #[serde(default)]
    pub pushover_user_key: String,
    #[serde(default)]
    pub pushover_api_token: String,
    #[serde(default)]
    pub pushplus_token: String,
    #[serde(default)]
    pub serverchan_token: String,
    #[serde(default)]
    pub ntfy_topic: String,
    #[serde(default)]
    pub ntfy_server: String,
    #[serde(default)]
    pub gotify_server: String,
    #[serde(default)]
    pub gotify_app_token: String,
    #[serde(default)]
    pub custom_webhook_url: String,
    #[serde(default = "default_quiet_hours_start")]
    pub quiet_hours_start: i32,
    #[serde(default = "default_quiet_hours_end")]
    pub quiet_hours_end: i32,
    #[serde(default = "default_dedup_window")]
    pub dedup_window_minutes: i32,
    #[serde(default)]
    pub feishu_app_id: String,
    #[serde(default)]
    pub feishu_app_secret: String,
    #[serde(default)]
    pub feishu_chat_id: String,
    #[serde(default)]
    pub feishu_receive_id_type: String,
    #[serde(default)]
    pub feishu_domain: String,
    #[serde(default)]
    pub feishu_folder_token: String,
    #[serde(default)]
    pub feishu_verification_token: String,
    #[serde(default)]
    pub feishu_encrypt_key: String,
    #[serde(default)]
    pub feishu_stream_enabled: bool,
    #[serde(default)]
    pub feishu_webhook_secret: String,
    #[serde(default)]
    pub dingtalk_app_key: String,
    #[serde(default)]
    pub dingtalk_app_secret: String,
    #[serde(default)]
    pub dingtalk_stream_enabled: bool,
    #[serde(default)]
    pub dingtalk_secret: String,
    #[serde(default)]
    pub discord_bot_token: String,
    #[serde(default)]
    pub discord_main_channel_id: String,
    #[serde(default)]
    pub slack_bot_token: String,
    #[serde(default)]
    pub slack_channel_id: String,
    #[serde(default)]
    pub wecom_corpid: String,
    #[serde(default)]
    pub wecom_token: String,
    #[serde(default)]
    pub wecom_encoding_aes_key: String,
    #[serde(default)]
    pub wecom_agent_id: String,
    #[serde(default)]
    pub custom_webhook_bearer_token: String,
    #[serde(default)]
    pub custom_webhook_body_template: String,
    #[serde(default = "default_true")]
    pub webhook_verify_ssl: bool,
    #[serde(default)]
    pub report_channels: Vec<String>,
    #[serde(default)]
    pub alert_channels: Vec<String>,
    #[serde(default)]
    pub system_error_channels: Vec<String>,
    #[serde(default = "default_dedup_ttl")]
    pub dedup_ttl_seconds: u64,
    #[serde(default = "default_cooldown_seconds")]
    pub cooldown_seconds: u64,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(default = "default_min_severity")]
    pub min_severity: String,
    #[serde(default)]
    pub daily_digest_enabled: bool,
}

/// 搜索引擎配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    #[serde(default)]
    pub default_provider: String,
    #[serde(default)]
    pub serper_api_key: String,
    #[serde(default)]
    pub serper_api_key_env: String,
    #[serde(default)]
    pub bing_api_key: String,
    #[serde(default)]
    pub bing_api_key_env: String,
    #[serde(default)]
    pub google_api_key: String,
    #[serde(default)]
    pub google_api_key_env: String,
    #[serde(default)]
    pub google_cx: String,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            default_provider: "serper".to_string(),
            serper_api_key: String::new(),
            serper_api_key_env: "SERPER_API_KEY".to_string(),
            bing_api_key: String::new(),
            bing_api_key_env: "BING_SEARCH_API_KEY".to_string(),
            google_api_key: String::new(),
            google_api_key_env: "GOOGLE_SEARCH_API_KEY".to_string(),
            google_cx: String::new(),
        }
    }
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            dingtalk_webhook: String::new(),
            feishu_webhook: String::new(),
            wecom_webhook: String::new(),
            telegram_bot_token: String::new(),
            telegram_chat_id: String::new(),
            bark_url: String::new(),
            email_smtp_host: String::new(),
            email_smtp_port: 465,
            email_user: String::new(),
            email_pass: String::new(),
            email_pass_env: String::new(),
            email_from: String::new(),
            email_to: String::new(),
            discord_webhook: String::new(),
            slack_webhook: String::new(),
            pushover_user_key: String::new(),
            pushover_api_token: String::new(),
            pushplus_token: String::new(),
            serverchan_token: String::new(),
            ntfy_topic: String::new(),
            ntfy_server: String::new(),
            gotify_server: String::new(),
            gotify_app_token: String::new(),
            custom_webhook_url: String::new(),
            quiet_hours_start: default_quiet_hours_start(),
            quiet_hours_end: default_quiet_hours_end(),
            dedup_window_minutes: default_dedup_window(),
            feishu_app_id: String::new(),
            feishu_app_secret: String::new(),
            feishu_chat_id: String::new(),
            feishu_receive_id_type: String::new(),
            feishu_domain: String::new(),
            feishu_folder_token: String::new(),
            feishu_verification_token: String::new(),
            feishu_encrypt_key: String::new(),
            feishu_stream_enabled: false,
            feishu_webhook_secret: String::new(),
            dingtalk_app_key: String::new(),
            dingtalk_app_secret: String::new(),
            dingtalk_stream_enabled: false,
            dingtalk_secret: String::new(),
            discord_bot_token: String::new(),
            discord_main_channel_id: String::new(),
            slack_bot_token: String::new(),
            slack_channel_id: String::new(),
            wecom_corpid: String::new(),
            wecom_token: String::new(),
            wecom_encoding_aes_key: String::new(),
            wecom_agent_id: String::new(),
            custom_webhook_bearer_token: String::new(),
            custom_webhook_body_template: String::new(),
            webhook_verify_ssl: true,
            report_channels: vec![],
            alert_channels: vec![],
            system_error_channels: vec![],
            dedup_ttl_seconds: default_dedup_ttl(),
            cooldown_seconds: default_cooldown_seconds(),
            timezone: default_timezone(),
            min_severity: default_min_severity(),
            daily_digest_enabled: false,
        }
    }
}

/// 代理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    #[serde(default)]
    pub http_proxy: String,
    #[serde(default)]
    pub https_proxy: String,
    #[serde(default)]
    pub no_proxy: String,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            http_proxy: String::new(),
            https_proxy: String::new(),
            no_proxy: String::new(),
        }
    }
}

/// 社交情绪采集配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialSentimentConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub eastmoney_board_url: String,
}

impl Default for SocialSentimentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            eastmoney_board_url: String::new(),
        }
    }
}

/// 新闻采集配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsConfig {
    #[serde(default = "default_news_max_age")]
    pub max_age_days: u32,
    #[serde(default = "default_news_strategy_profile")]
    pub strategy_profile: String,
    #[serde(default = "default_news_retention")]
    pub intel_retention_days: u32,
    #[serde(default = "default_timeout")]
    pub fetch_timeout_sec: u64,
    #[serde(default = "default_news_max_items")]
    pub max_items_per_source: u32,
    #[serde(default = "default_true")]
    pub auto_fetch_enabled: bool,
    #[serde(default)]
    pub newsnow_base_url: String,
}

impl Default for NewsConfig {
    fn default() -> Self {
        Self {
            max_age_days: default_news_max_age(),
            strategy_profile: default_news_strategy_profile(),
            intel_retention_days: default_news_retention(),
            fetch_timeout_sec: default_timeout(),
            max_items_per_source: default_news_max_items(),
            auto_fetch_enabled: true,
            newsnow_base_url: String::new(),
        }
    }
}

/// 报告生成配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportConfig {
    #[serde(default)]
    pub templates_dir: String,
    #[serde(default = "default_true")]
    pub renderer_enabled: bool,
    #[serde(default = "default_report_language")]
    pub report_language: String,
    #[serde(default)]
    pub summary_only: bool,
    #[serde(default)]
    pub show_llm_model: bool,
    #[serde(default = "default_report_type")]
    pub default_report_type: String,
    #[serde(default = "default_true")]
    pub integrity_enabled: bool,
    #[serde(default = "default_integrity_retry")]
    pub integrity_retry: u32,
    #[serde(default = "default_history_compare_n")]
    pub history_compare_n: u32,
}

impl Default for ReportConfig {
    fn default() -> Self {
        Self {
            templates_dir: String::new(),
            renderer_enabled: true,
            report_language: default_report_language(),
            summary_only: false,
            show_llm_model: false,
            default_report_type: default_report_type(),
            integrity_enabled: true,
            integrity_retry: default_integrity_retry(),
            history_compare_n: default_history_compare_n(),
        }
    }
}

/// 机器人交互配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_command_prefix")]
    pub command_prefix: String,
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_minute: u32,
    #[serde(default)]
    pub admin_users: Vec<String>,
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            command_prefix: default_command_prefix(),
            rate_limit_per_minute: default_rate_limit(),
            admin_users: vec![],
        }
    }
}

/// 组合风控配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioRiskConfig {
    #[serde(default = "default_concentration_pct")]
    pub concentration_alert_pct: f64,
    #[serde(default = "default_drawdown_pct")]
    pub drawdown_alert_pct: f64,
    #[serde(default = "default_stop_loss_pct")]
    pub stop_loss_alert_pct: f64,
    #[serde(default = "default_sector_concentration_pct")]
    pub sector_concentration_alert_pct: f64,
}

impl Default for PortfolioRiskConfig {
    fn default() -> Self {
        Self {
            concentration_alert_pct: default_concentration_pct(),
            drawdown_alert_pct: default_drawdown_pct(),
            stop_loss_alert_pct: default_stop_loss_pct(),
            sector_concentration_alert_pct: default_sector_concentration_pct(),
        }
    }
}

/// 第三方数据源配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceConfig {
    #[serde(default)]
    pub tushare_token: String,
    #[serde(default)]
    pub tushare_token_env: String,
    #[serde(default)]
    pub finnhub_api_key: String,
    #[serde(default)]
    pub finnhub_api_key_env: String,
    #[serde(default)]
    pub alphavantage_api_key: String,
    #[serde(default)]
    pub alphavantage_api_key_env: String,
    #[serde(default)]
    pub longbridge_app_key: String,
    #[serde(default)]
    pub longbridge_app_secret: String,
    #[serde(default)]
    pub longbridge_access_token: String,
}

impl Default for DataSourceConfig {
    fn default() -> Self {
        Self {
            tushare_token: String::new(),
            tushare_token_env: String::new(),
            finnhub_api_key: String::new(),
            finnhub_api_key_env: String::new(),
            alphavantage_api_key: String::new(),
            alphavantage_api_key_env: String::new(),
            longbridge_app_key: String::new(),
            longbridge_app_secret: String::new(),
            longbridge_access_token: String::new(),
        }
    }
}

/// HTTP服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    #[serde(default = "default_cors_origins")]
    pub cors_origins: Vec<String>,
    #[serde(default)]
    pub auth_password: String,
    #[serde(default)]
    pub auth_password_env: String,
}

/// 股票监控配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockConfig {
    #[serde(default)]
    pub watchlist: Vec<String>,
    #[serde(default = "default_true")]
    pub trading_day_check: bool,
    #[serde(default = "default_true")]
    pub enable_realtime: bool,
    #[serde(default = "default_true")]
    pub enable_chip_distribution: bool,
    #[serde(default)]
    pub realtime_source_priority: Vec<String>,
}

/// LLM通道配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmChannelConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub api_key_env: String,
    #[serde(default)]
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for LlmChannelConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            provider: String::new(),
            api_key_env: String::new(),
            model: String::new(),
            temperature: default_temperature(),
            timeout_seconds: default_timeout(),
            enabled: true,
        }
    }
}

/// LLM大语言模型配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub api_key_env: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default)]
    pub max_tokens: u32,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub fallback_provider: Option<String>,
    #[serde(default)]
    pub fallback_api_key_env: Option<String>,
    #[serde(default)]
    pub fallback_model: Option<String>,
    #[serde(default)]
    pub channels: Vec<LlmChannelConfig>,
    #[serde(default)]
    pub litellm_config_path: String,
    #[serde(default)]
    pub fallback_models: Vec<String>,
    #[serde(default)]
    pub vision_model: String,
    #[serde(default)]
    pub vision_provider: String,
    #[serde(default = "default_true")]
    pub prompt_cache_telemetry_enabled: bool,
    #[serde(default)]
    pub prompt_cache_diagnostics_level: String,
}

/// 数据库连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_type")]
    pub db_type: String,
    pub host: String,
    pub port: u16,
    pub name: String,
    pub user: String,
    #[serde(default)]
    pub password_env: String,
    #[serde(default)]
    pub password: String,
}

fn default_db_type() -> String { "mysql".to_string() }

impl DatabaseConfig {
    pub fn is_sqlite(&self) -> bool {
        self.db_type.to_lowercase() == "sqlite"
    }
}

/// 定时调度配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_schedule_times")]
    pub times: Vec<String>,
    #[serde(default = "default_true")]
    pub run_immediately: bool,
}

/// 市场复盘配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketReviewConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_region")]
    pub region: String,
}

/// 回测配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_eval_window")]
    pub eval_window_days: u32,
    #[serde(default = "default_min_age")]
    pub min_age_days: u32,
    #[serde(default = "default_neutral_band")]
    pub neutral_band_pct: f64,
}

/// Agent智能体配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default = "default_agent_max_steps")]
    pub max_steps: u32,
    #[serde(default = "default_agent_arch")]
    pub arch: String,
    #[serde(default = "default_agent_orchestrator_mode")]
    pub orchestrator_mode: String,
    #[serde(default)]
    pub context_compression_enabled: bool,
    #[serde(default = "default_compression_trigger")]
    pub context_compression_trigger_tokens: u32,
    #[serde(default = "default_protected_turns")]
    pub context_protected_turns: u32,
    #[serde(default = "default_deep_research_budget")]
    pub deep_research_budget: u32,
    #[serde(default = "default_timeout")]
    pub deep_research_timeout: u64,
    #[serde(default)]
    pub memory_enabled: bool,
    #[serde(default)]
    pub skill_autoweight: bool,
    #[serde(default)]
    pub nl_routing: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8000,
                cors_origins: default_cors_origins(),
                auth_password: String::new(),
                auth_password_env: String::new(),
            },
            stock: StockConfig {
                watchlist: vec!["600519".to_string(), "300750".to_string(), "002594".to_string()],
                trading_day_check: true,
                enable_realtime: true,
                enable_chip_distribution: true,
                realtime_source_priority: vec![
                    "tencent".to_string(),
                    "sina".to_string(),
                    "eastmoney".to_string(),
                ],
            },
            llm: LlmConfig {
                provider: "deepseek".to_string(),
                api_key: String::new(),
                api_key_env: "DEEPSEEK_API_KEY".to_string(),
                model: default_model(),
                temperature: default_temperature(),
                timeout_seconds: default_timeout(),
                max_tokens: 4096,
                base_url: String::new(),
                fallback_provider: None,
                fallback_api_key_env: None,
                fallback_model: None,
                channels: vec![],
                litellm_config_path: String::new(),
                fallback_models: vec![],
                vision_model: String::new(),
                vision_provider: String::new(),
                prompt_cache_telemetry_enabled: true,
                prompt_cache_diagnostics_level: String::new(),
            },
            database: DatabaseConfig {
                db_type: default_db_type(),
                host: "127.0.0.1".to_string(),
                port: 3306,
                name: "dsa".to_string(),
                user: "root".to_string(),
                password_env: "DSA_DB_PASSWORD".to_string(),
                password: String::new(),
            },
            scheduler: SchedulerConfig {
                enabled: false,
                times: default_schedule_times(),
                run_immediately: true,
            },
            market_review: MarketReviewConfig {
                enabled: true,
                region: default_region(),
            },
            backtest: BacktestConfig {
                enabled: true,
                eval_window_days: default_eval_window(),
                min_age_days: default_min_age(),
                neutral_band_pct: default_neutral_band(),
            },
            agent: AgentConfig {
                enabled: false,
                skills: vec![],
                max_steps: default_agent_max_steps(),
                arch: default_agent_arch(),
                orchestrator_mode: default_agent_orchestrator_mode(),
                context_compression_enabled: false,
                context_compression_trigger_tokens: default_compression_trigger(),
                context_protected_turns: default_protected_turns(),
                deep_research_budget: default_deep_research_budget(),
                deep_research_timeout: default_timeout(),
                memory_enabled: false,
                skill_autoweight: false,
                nl_routing: false,
            },
            notification: NotificationConfig::default(),
            search: SearchConfig::default(),
            proxy: ProxyConfig::default(),
            social_sentiment: SocialSentimentConfig::default(),
            news: NewsConfig::default(),
            report: ReportConfig::default(),
            bot: BotConfig::default(),
            portfolio_risk: PortfolioRiskConfig::default(),
            data_source: DataSourceConfig::default(),
        }
    }
}

impl AppConfig {
    /// 从TOML文件加载配置，文件不存在则返回默认值
    pub fn load(path: &Path) -> crate::errors::DsaResult<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::DsaError::Config(format!("读取配置文件失败: {}", e)))?;

        let config: Self = toml::from_str(&content)
            .map_err(|e| crate::DsaError::Config(format!("解析配置文件失败: {}", e)))?;
        Ok(config)
    }

    /// 从环境变量解析LLM API密钥
    pub fn resolve_api_key(&self) -> String {
        if !self.llm.api_key.is_empty() {
            return self.llm.api_key.clone();
        }
        if !self.llm.api_key_env.is_empty() {
            if let Ok(key) = std::env::var(&self.llm.api_key_env) {
                return key;
            }
        }
        String::new()
    }

    /// 从环境变量或配置解析数据库密码
    ///
    /// 优先从 password_env 指定的环境变量读取，找不到则回退到 password 字段
    pub fn resolve_db_password(&self) -> String {
        if !self.database.password_env.is_empty() {
            if let Ok(pwd) = std::env::var(&self.database.password_env) {
                return pwd;
            }
        }
        self.database.password.clone()
    }

    /// 生成MySQL连接DSN字符串
    pub fn mysql_dsn(&self) -> String {
        let pwd = self.resolve_db_password();
        format!(
            "mysql://{}:{}@{}:{}/{}",
            self.database.user, pwd, self.database.host, self.database.port, self.database.name
        )
    }

    /// 生成SQLite数据库文件路径
    /// 如果 name 是绝对路径则直接使用，否则相对于当前目录下的 data/ 目录
    /// 自动创建 data/ 目录（如果不存在）
    pub fn sqlite_path(&self) -> String {
        let name = &self.database.name;
        let path = if name.starts_with('/') || name.starts_with("./") || name.contains('/') {
            name.clone()
        } else {
            format!("data/{}.db", name)
        };
        if let Some(parent) = std::path::Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        path
    }

    /// 根据配置构建数据库连接器
    pub fn build_connector(&self) -> deck_connector::Connector {
        if self.database.is_sqlite() {
            deck_connector::Connector::new("sqlite").db(&self.sqlite_path())
        } else {
            let password = self.resolve_db_password();
            deck_connector::Connector::new("mysql")
                .server(&self.database.host)
                .port(self.database.port)
                .user(&self.database.user)
                .password(&password)
                .db(&self.database.name)
        }
    }
}

fn default_cors_origins() -> Vec<String> {
    vec![
        "http://localhost:5173".to_string(),
        "http://127.0.0.1:5173".to_string(),
    ]
}

fn default_true() -> bool { true }
fn default_model() -> String { "deepseek-chat".to_string() }
fn default_temperature() -> f64 { 0.7 }
fn default_timeout() -> u64 { 60 }
fn default_schedule_times() -> Vec<String> { vec!["18:00".to_string()] }
fn default_region() -> String { "cn".to_string() }
fn default_eval_window() -> u32 { 10 }
fn default_min_age() -> u32 { 14 }
fn default_neutral_band() -> f64 { 2.0 }
fn default_agent_max_steps() -> u32 { 10 }
fn default_agent_arch() -> String { "single".to_string() }
fn default_agent_orchestrator_mode() -> String { "standard".to_string() }
fn default_quiet_hours_start() -> i32 { 23 }
fn default_quiet_hours_end() -> i32 { 7 }
fn default_dedup_window() -> i32 { 30 }
fn default_dedup_ttl() -> u64 { 600 }
fn default_cooldown_seconds() -> u64 { 300 }
fn default_timezone() -> String { "Asia/Shanghai".to_string() }
fn default_min_severity() -> String { "info".to_string() }
fn default_news_max_age() -> u32 { 3 }
fn default_news_strategy_profile() -> String { "default".to_string() }
fn default_news_retention() -> u32 { 90 }
fn default_news_max_items() -> u32 { 20 }
fn default_report_language() -> String { "zh".to_string() }
fn default_report_type() -> String { "full".to_string() }
fn default_integrity_retry() -> u32 { 2 }
fn default_history_compare_n() -> u32 { 5 }
fn default_command_prefix() -> String { "/".to_string() }
fn default_rate_limit() -> u32 { 10 }
fn default_concentration_pct() -> f64 { 30.0 }
fn default_drawdown_pct() -> f64 { 15.0 }
fn default_stop_loss_pct() -> f64 { 8.0 }
fn default_sector_concentration_pct() -> f64 { 40.0 }
fn default_compression_trigger() -> u32 { 8000 }
fn default_protected_turns() -> u32 { 4 }
fn default_deep_research_budget() -> u32 { 5 }
