//! DSA Service - 业务服务层
//!
//! 包含所有业务服务模块, 每个服务通过 `dispatch(method, params)` 统一分发请求

#[macro_use]
extern crate tube;

/// 全局配置缓存
pub mod config;
/// 股票数据服务 - 行情/K线/搜索/基本面
pub mod stock_service;
/// 分析服务 - AI分析/报告/历史
pub mod analysis_service;
/// 调度服务 - 定时分析任务
pub mod scheduler_service;
/// 组合管理服务 - 持仓/交易/快照
pub mod portfolio_service;
/// 大盘/市场服务 - 指数/板块/热门
pub mod market_service;
/// 决策信号服务 - 信号创建/追踪/评估
pub mod decision_service;
/// 情报源服务 - RSS/API数据源管理
pub mod intelligence_service;
/// 告警服务 - 规则/触发/通知
pub mod alert_service;
/// LLM使用量追踪服务
pub mod usage_service;
/// 基础认证服务
pub mod auth_service;
/// 系统配置服务
pub mod system_service;
/// AlphaSift筛选服务 - 策略/热点
pub mod screening_service;
/// 通知服务 - 多渠道消息推送
pub mod notification_service;
/// 搜索服务 - Web搜索集成
pub mod search_service;
/// 社交情绪服务 - 恐惧贪婪/个股情绪
pub mod social_sentiment_service;
/// 报告服务 - 模板渲染/i18n
pub mod report_service;
/// 回测服务 - 信号回测评估
pub mod backtest_service;
/// 告警工人服务 - 评估/指标/信号灯
pub mod alert_worker_service;
/// 决策信号提取服务 - 从分析报告提取信号
pub mod decision_extractor_service;
/// 市场上下文服务 - 阶段判断/护栏检查
pub mod market_context_service;
/// 名称解析服务 - 股票代码/名称查询
pub mod name_resolver_service;
/// Bot服务
pub mod bot;

/// 股票数据服务
pub use stock_service::StockService;
/// 分析服务
pub use analysis_service::AnalysisService;
/// 调度服务
pub use scheduler_service::SchedulerService;
/// 组合管理服务
pub use portfolio_service::PortfolioService;
/// 市场服务
pub use market_service::MarketService;
/// 决策信号服务
pub use decision_service::DecisionService;
/// 情报源服务
pub use intelligence_service::IntelligenceService;
/// 告警服务
pub use alert_service::AlertService;
/// LLM使用量追踪服务
pub use usage_service::UsageService;
/// 认证服务
pub use auth_service::AuthService;
/// 系统配置服务
pub use system_service::SystemService;
/// AlphaSift筛选服务
pub use screening_service::ScreeningService;
/// 通知服务
pub use notification_service::NotificationService;
/// 搜索服务
pub use search_service::SearchService;
/// 社交情绪服务
pub use social_sentiment_service::SocialSentimentService;
/// 报告服务
pub use report_service::ReportService;
/// 回测服务
pub use backtest_service::BacktestService;
/// 告警工人服务
pub use alert_worker_service::AlertWorkerService;
/// 决策信号提取服务
pub use decision_extractor_service::DecisionExtractorService;
/// 市场上下文服务
pub use market_context_service::MarketContextService;
/// 名称解析服务
pub use name_resolver_service::NameResolverService;
/// Bot调度器
pub use bot::dispatcher::BotDispatcher;
