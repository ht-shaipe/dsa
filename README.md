# DSA - Daily Stock Analysis

AI 驱动的每日股票分析系统，基于 Rust + Vue 技术栈。提供技术指标分析、LLM 智能解读、大盘综述、策略回测、多 Agent 对话、组合管理、告警推送、Bot 交互等完整能力。同时支持 Web 部署和 Tauri 桌面应用。

## 功能特性

- **AI 分析**: LLM 驱动的个股分析报告与大盘综述，支持 DeepSeek / OpenAI / Anthropic 等模型
- **技术指标**: MA / MACD / KDJ / RSI / BOLL 等主流指标计算与可视化
- **大盘综述**: 自动生成 A 股市场日评，含板块涨跌与资金流向
- **策略回测**: 基于分析信号的回测引擎，评估窗口与中性带可配
- **Agent 对话**: 多 Agent 编排器 + ReAct 执行器 + 工具调用，支持技能路由
- **策略筛选**: 多维度条件筛选股票，预设策略 + 自定义条件
- **组合管理**: 投资组合创建、交易记录、持仓跟踪与绩效评估
- **决策信号**: 自动提取买卖决策建议，信号追踪与用户反馈
- **告警系统**: 价格 / 指标告警，支持冷却去重、静默时段与路由规则
- **情报中心**: 新闻与社交情绪聚合
- **通知推送**: 14 种渠道全覆盖，含路由分级与静默策略
- **Bot 平台**: 钉钉 / 飞书 / Discord 机器人交互
- **数据源**: 东方财富 / 腾讯 / 新浪 / 通达信 / Tushare / Finnhub / AlphaVantage / Longbridge
- **桌面应用**: Tauri 桌面端，内嵌后端服务，支持自动更新与系统托盘

## 技术架构

```
dsa/
├── Cargo.toml              # workspace 根配置
├── Makefile                # 常用开发命令快捷入口
├── conf/
│   └── config.toml         # 运行时配置
├── crates/
│   ├── dsa-core/           # 基础层: 模型 / 配置 / 错误 / DB (29 个实体)
│   ├── dsa-pipeline/       # 管道层: 技术分析 / LLM 调用 / 报告生成
│   ├── dsa-agent/          # Agent 层: 编排器 / 技能路由 / ReAct 执行 / 工具集
│   ├── dsa-backtest/       # 回测层: 引擎 / 信号 / 报告
│   ├── dsa-service/        # 业务层: 23 个服务模块
│   ├── dsa-server/         # 接口层: Actix-web HTTP API (模块路由分发)
│   └── dsa-app/            # 桌面层: Tauri 应用 (内嵌 Actix-web 服务)
└── web/                    # 前端: Vue 3 + Vite + TypeScript
    └── src/
        ├── api/            # API 调用封装 (18 模块)
        ├── assets/         # 静态资源
        ├── components/     # 通用组件 (K线图/Markdown/仪表盘/搜索)
        ├── composables/    # 组合式函数 (交易时段/自动更新)
        ├── layout/         # 布局组件
        ├── router/         # 路由配置
        ├── stores/         # Pinia 状态管理
        ├── styles/         # 全局样式
        └── views/          # 页面视图 (12 页)
```

### 后端技术栈

| 类别 | 技术 |
|------|------|
| Web 框架 | Actix-web 4 |
| ORM | deck (MySQL / SQLite，自动迁移) |
| 行情数据 | qta_crawler / qta_tdx / qta_sdk |
| LLM | ai-llm-kit |
| 工具库 | tube (derive / utils / fs / cmd / net / web / value / jwt) |
| 桌面框架 | Tauri 2 |
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
| **dsa-core** | 配置加载、数据模型 (29 实体)、错误定义、数据库连接与迁移、工具函数 |
| **dsa-pipeline** | 技术指标计算、LLM 调用管道、上下文构建、报告渲染、Prompt 模板、大盘综述 |
| **dsa-agent** | 多 Agent 编排 (8 专用 Agent)、技能路由、ReAct 执行器、对话管理、记忆、9 类工具注册 |
| **dsa-backtest** | 回测引擎、信号评估与追踪、回测报告生成 |
| **dsa-service** | 23 个业务服务 (stock / analysis / market / agent / backtest / scheduler / portfolio / screening / decision / intelligence / alert / notification / search / social_sentiment / report / config / usage / system / bot / indicator / name_resolver / market_context / decision_extractor / alert_worker) |
| **dsa-server** | HTTP API 入口、模块路由分发、REST 认证接口、SSE 流、静态文件托管 |
| **dsa-app** | Tauri 桌面应用、内嵌 Actix-web (端口 18080)、系统托盘、自动更新 |

## 快速开始

```bash
# 克隆项目
git clone <repo-url> && cd dsa

# 一键开发环境 (需 MySQL)
make env          # 生成 .env 文件
make db-init      # 初始化数据库
make dev          # 启动后端 + 前端

# 或手动启动
cargo build       # 编译后端
cd web && npm install && npm run dev  # 安装并启动前端
cargo run --bin dsa                   # 启动后端
```

> 详细的环境准备、配置和部署说明请参阅 [docs/GETTING_STARTED.md](docs/GETTING_STARTED.md)

## 前端页面

| 路由 | 页面 | 说明 |
|------|------|------|
| `/` | DashboardView | 仪表盘概览 (大盘指数 + 个股分析) |
| `/watchlist` | WatchlistView | 关注列表管理 |
| `/analysis-history` | AnalysisHistoryView | 分析历史记录 |
| `/chat` | ChatView | Agent 对话 (SSE 流式) |
| `/screening` | ScreeningView | 策略筛选 |
| `/portfolio` | PortfolioView | 组合管理 |
| `/decision-signals` | DecisionSignalsView | 决策信号 |
| `/backtest` | BacktestView | 策略回测 |
| `/alerts` | AlertsView | 告警管理 |
| `/usage` | UsageView | 用量统计 |
| `/settings` | SettingsView | 系统设置 |
| `/login` | LoginView | 登录认证 |

## 文档

| 文档 | 说明 |
|------|------|
| [GETTING_STARTED.md](docs/GETTING_STARTED.md) | 从零搭建和使用指南 |
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | 系统架构详细说明 |
| [API.md](docs/API.md) | API 接口参考 |
| [DEVELOPMENT.md](docs/DEVELOPMENT.md) | 开发指南 |

## License

MIT
