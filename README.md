# DSA - Daily Stock Analysis (Rust + Vue 版)

AI 驱动的每日股票分析系统，从 Python 迁移至 Rust + Vue 技术栈。提供技术指标分析、LLM 智能解读、大盘综述、策略回测、多 Agent 对话、组合管理、告警推送、Bot 交互等完整能力。

## 功能特性

- **AI 分析**: LLM 驱动的个股分析报告与大盘综述，支持 DeepSeek 等多种模型
- **技术指标**: MA/MACD/KDJ/RSI/BOLL 等主流指标计算与可视化
- **大盘综述**: 自动生成 A 股市场日评，含板块涨跌与资金流向
- **策略回测**: 基于分析信号的回测引擎，评估窗口与中性带可配
- **Agent 对话**: 多 Agent 编排器 + ReAct 执行器 + 工具调用，支持技能路由
- **策略筛选**: 多维度条件筛选股票
- **组合管理**: 投资组合创建、跟踪与评估
- **决策信号**: 自动提取买卖决策建议
- **告警系统**: 价格/指标告警，支持静默时段与路由规则
- **情报中心**: 新闻与社交情绪聚合
- **通知推送**: 14 种渠道全覆盖，含路由分级与静默策略
- **Bot 平台**: 钉钉/飞书/Discord 机器人交互
- **数据源**: 东方财富/腾讯/新浪/通达信/Tushare/Finnhub/AlphaVantage/Longbridge

## 技术架构

```
dsa/
├── Cargo.toml              # workspace 根配置
├── conf/
│   └── config.toml         # 运行时配置
├── crates/
│   ├── dsa-core/           # 基础层: 模型/配置/错误/DB
│   ├── dsa-pipeline/       # 管道层: 技术分析/LLM调用/报告生成
│   ├── dsa-agent/          # Agent层: 编排器/技能路由/ReAct执行/工具集
│   ├── dsa-backtest/       # 回测层: 引擎/信号/报告
│   ├── dsa-service/        # 业务层: 24个服务模块
│   └── dsa-server/         # 接口层: Actix-web HTTP API (soma模式)
└── web/                    # 前端: Vue3 + Vite + TypeScript
    └── src/
        ├── api/            # API 调用封装
        ├── components/     # 通用组件
        ├── layout/         # 布局组件
        ├── router/         # 路由配置
        ├── stores/         # Pinia 状态管理
        ├── styles/         # 全局样式
        └── views/          # 页面视图
```

### 后端技术栈

| 类别 | 技术 |
|------|------|
| Web 框架 | Actix-web 4 |
| ORM | deck (MySQL, 自动迁移) |
| 行情数据 | qta_crawler / qta_tdx / qta_sdk |
| LLM | ai-llm-kit |
| 工具库 | tube (derive/utils/fs/cmd/net/web/value) |
| 异步运行时 | Tokio |
| 序列化 | serde / serde_json |
| 配置 | toml |

### 前端技术栈

| 类别 | 技术 |
|------|------|
| 框架 | Vue 3.5 |
| 构建 | Vite 6 |
| 语言 | TypeScript 5.7 |
| UI 组件 | Element Plus 2.9 |
| 图表 | ECharts 5.6 + vue-echarts 7 |
| 状态管理 | Pinia 3 |
| 路由 | Vue Router 4 |
| HTTP | Axios |
| Markdown | markdown-it + highlight.js |

### Workspace Crate 职责

| Crate | 职责 |
|-------|------|
| **dsa-core** | 配置加载、数据模型、错误定义、数据库连接与迁移、工具函数 |
| **dsa-pipeline** | 技术指标计算、LLM 调用管道、上下文构建、报告渲染、Prompt 模板 |
| **dsa-agent** | 多 Agent 编排、技能路由、ReAct 执行器、对话管理、记忆、工具注册 |
| **dsa-backtest** | 回测引擎、信号评估、回测报告生成 |
| **dsa-service** | 24 个业务服务实现 (stock/analysis/market/agent/backtest/scheduler/...) |
| **dsa-server** | HTTP API 入口、soma 模式路由分发、认证中间件、SSE 流 |

## 快速开始

### 环境要求

- Rust 1.75+ (edition 2021)
- Node.js 18+
- MySQL 8.0+
- 操作系统: macOS / Linux

### 安装

```bash
# 克隆项目
git clone <repo-url> && cd dsa

# 后端编译
cargo build --release

# 前端安装
cd web && npm install && cd ..
```

### 配置

复制并编辑配置文件:

```bash
cp conf/config.toml conf/config.local.toml
```

关键配置项:

```toml
[database]
host = "127.0.0.1"
port = 3306
name = "dsa"
user = "root"
password = ""

[llm]
provider = "deepseek"
model = "deepseek-chat"
api_key_env = "DEEPSEEK_API_KEY"
```

数据库会在首次启动时由 deck ORM 自动创建表结构，无需手动建表。

### 启动

```bash
# 启动后端 (默认 0.0.0.0:8000)
cargo run --bin dsa-server

# 启动前端开发服务器 (默认 localhost:5173)
cd web && npm run dev
```

生产环境构建前端:

```bash
cd web && npm run build
# 产物输出至 web/dist/，可由 Actix-web 静态文件服务托管
```

## 配置说明

配置文件路径: `conf/config.toml`

| 配置段 | 说明 | 关键字段 |
|--------|------|----------|
| `[server]` | HTTP 服务配置 | host, port, cors_origins |
| `[stock]` | 股票数据配置 | enable_realtime, realtime_source_priority, watchlist, trading_day_check |
| `[llm]` | LLM 模型配置 | provider, model, api_key_env, temperature, timeout_seconds |
| `[database]` | MySQL 连接配置 | host, port, name, user, password, password_env |
| `[scheduler]` | 定时任务配置 | enabled, times, run_immediately |
| `[market_review]` | 大盘综述配置 | enabled, region |
| `[backtest]` | 回测引擎配置 | enabled, eval_window_days, min_age_days, neutral_band_pct |
| `[agent]` | Agent 配置 | enabled, arch, orchestrator_mode, max_steps, skills |
| `[notification]` | 通知渠道配置 | dingtalk_webhook, feishu_webhook, wecom_webhook, telegram_bot_token, bark_url, email_* |
| `[search]` | 搜索引擎配置 | default_provider, serper_api_key, bing_api_key, google_api_key |

敏感信息支持环境变量引用: `api_key_env` / `password_env` 字段指定环境变量名，优先从环境变量读取。

## API 接口一览

所有接口遵循 soma 模式: `POST /api/v1/{module}`，通过请求体中的 `method` 字段路由到具体方法。

```
POST /api/v1/{module}
Content-Type: application/json

{
  "method": "list",
  "params": { ... }
}
```

### 25 个 API 模块

| 模块 | 说明 | 主要方法 |
|------|------|----------|
| stock | 股票数据 | quote, history, realtime, search, watchlist |
| analysis | 个股分析 | analyze, report, list, latest |
| market | 市场数据 | overview, sectors, indices, review |
| market_context | 市场上下文 | get, refresh |
| agent | Agent 对话 | chat, stream, history, sessions |
| backtest | 策略回测 | run, list, detail, compare |
| backtest_worker | 回测任务 | queue, status, cancel |
| scheduler | 定时调度 | status, trigger, config |
| portfolio | 组合管理 | create, list, detail, update, performance |
| screening | 策略筛选 | screen, presets, save |
| decision | 决策信号 | list, detail, stats |
| decision_extractor | 决策提取 | extract, batch |
| intelligence | 情报中心 | news, digest, search |
| alert | 告警管理 | create, list, update, delete, trigger |
| alert_worker | 告警任务 | check, status |
| notification | 通知推送 | send, channels, test, route |
| search | 搜索服务 | search, providers |
| social_sentiment | 社交情绪 | analyze, trending |
| name_resolver | 名称解析 | resolve, search |
| report | 报告管理 | generate, list, detail, export |
| config | 配置管理 | get, update, reload |
| auth | 认证授权 | login, status, logout |
| system | 系统信息 | info, health, stats |
| usage | 用量统计 | summary, detail, export |
| bot | Bot 平台 | dingtalk, feishu, discord, webhook |

### 通知渠道 (14 种)

| 渠道 | 配置字段 | 说明 |
|------|----------|------|
| 钉钉 | dingtalk_webhook | Webhook 机器人 |
| 飞书 | feishu_webhook | Webhook 机器人 |
| 企业微信 | wecom_webhook | Webhook 机器人 |
| Telegram | telegram_bot_token + telegram_chat_id | Bot API |
| Bark | bark_url | iOS 推送 |
| Email | email_smtp_host/port/user/pass/from/to | SMTP 邮件 |
| Discord | - | Webhook |
| Slack | - | Webhook |
| Pushover | - | 推送服务 |
| PushPlus | - | 微信推送 |
| ServerChan | - | Server酱 |
| ntfy | - | 开源推送 |
| Gotify | - | 自建推送 |
| 自定义 Webhook | - | 通用 HTTP |

路由规则: critical -> 全部已配置渠道, warning -> 即时通讯+邮件, info -> 仅日志。支持静默时段配置。

### Bot 平台

| 平台 | 说明 |
|------|------|
| 钉钉 | 出站 Webhook + 入站签名验证 |
| 飞书 | 出站 Webhook + 入站事件回调 |
| Discord | 出站 Webhook + 入站交互 |

### Agent 架构

```
orchestrator (standard / adaptive)
├── skill_agent        # 技能路由，分发到专业 Agent
├── technical_agent    # 技术分析专用
├── decision_agent     # 决策建议专用
├── risk_agent         # 风险评估专用
├── strategy_agent     # 策略生成专用
├── portfolio_agent    # 组合管理专用
├── intel_agent        # 情报分析专用
└── react_executor     # ReAct 循环执行器 (Thought → Action → Observation)
    └── tools/
        ├── data_tools        # 行情数据查询
        ├── analysis_tools    # 技术指标计算
        ├── market_tools      # 市场数据
        ├── backtest_tools    # 回测执行
        ├── chip_tools        # 筹码分布
        ├── pattern_tools     # 形态识别
        ├── portfolio_tools   # 组合查询
        ├── history_tools     # 历史数据
        └── search_tools      # 网络搜索
```

## 前端页面

| 路由 | 页面 | 说明 |
|------|------|------|
| `/` | DashboardView | 仪表盘概览 |
| `/chat` | ChatView | Agent 对话 (支持 SSE 流式) |
| `/screening` | ScreeningView | 策略筛选 |
| `/portfolio` | PortfolioView | 组合管理 |
| `/decision-signals` | DecisionSignalsView | 决策信号 |
| `/backtest` | BacktestView | 策略回测 |
| `/alerts` | AlertsView | 告警管理 |
| `/usage` | UsageView | 用量统计 |
| `/settings` | SettingsView | 系统设置 |
| `/login` | LoginView | 登录认证 |

状态管理 (Pinia Stores): `auth` (认证), `app` (全局), `analysis` (分析数据), `chat` (对话状态)

## 开发指南

### 添加新 Service

1. 在 `crates/dsa-service/src/` 下创建 `xxx_service.rs`
2. 实现 `XxxService` 结构体与 `dispatch(&self, method: &str, params: &Value) -> DsaResult<Value>` 方法
3. 在 `crates/dsa-service/src/lib.rs` 中注册模块

### 添加新 API 模块

1. 在 `crates/dsa-server/src/handler/` 下创建 `xxx.rs`
2. 实现 `distribute(param: &RequestParam) -> DsaResult<Value>` 函数，根据 `param.method` 分发到 service
3. 在 `crates/dsa-server/src/handler/mod.rs` 中声明模块
4. 在 `crates/dsa-server/src/router.rs` 的 `api_handler` match 中添加路由分支

### 添加前端页面

1. 在 `web/src/views/` 下创建 `XxxView.vue`
2. 在 `web/src/router/index.ts` 的 children 中添加路由
3. 如需状态管理，在 `web/src/stores/` 下创建对应 store
4. API 调用封装至 `web/src/api/`

### 添加 Agent 工具

1. 在 `crates/dsa-agent/src/tools/` 下创建 `xxx_tools.rs`
2. 实现工具函数，注册到 `register_all.rs`
3. 如需新 Agent，在 `crates/dsa-agent/src/agents/` 下添加，并在 `orchestrator.rs` 中接入

### 添加通知渠道

1. 在 `crates/dsa-service/src/notification_service.rs` 的 `send_to_channel` 中添加新渠道实现
2. 在 `crates/dsa-core/src/config.rs` 的 `NotificationConfig` 中添加配置字段
3. 更新 `conf/config.toml` 模板

## License

MIT
