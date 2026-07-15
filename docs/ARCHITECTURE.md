# DSA 系统架构

## 整体架构

DSA 采用分层架构设计，从底层数据模型到顶层接口逐步封装：

```
┌───────────────────────────────────────────────────────┐
│                    dsa-app (Tauri)                       │
│           桌面应用壳 / 系统托盘 / 自动更新              │
├────────────────────────────────────────────────────────┤
│                   dsa-server (Actix-web)                 │
│         HTTP API / SSE 流 / 认证 / 静态文件托管         │
├───────────────────────────────────────────────────────┤
│                    dsa-service                           │
│           23 个业务服务 / 通知推送 / Bot 调度            │
├──────────┬──────────┬──────────┬────────────────────────┤
│dsa-agent │dsa-back  │dsa-pipe  │                        │
│Agent编排  │回测引擎  │分析管道  │                        │
├──────────┴──────────┴──────────┴────────────────────────┤
│                     dsa-core                             │
│        配置 / 模型 / 错误 / DB 迁移 / 工具函数          │
└───────────────────────────────────────────────────────┘
```

---

## Crate 详解

### dsa-core — 基础层

公共模块：

| 模块 | 职责 |
|------|------|
| `config` | `AppConfig` 及所有子配置 (server / database / llm / scheduler / stock / market_review / backtest / agent / notification / search) |
| `db` | 数据库连接、`helper` (查询 / 执行工具)、`migration` (Schema 迁移) |
| `errors` | `DsaError` 枚举 (11 种变体: Config / StockData / LlmAnalysis / Database / ReportParse / Backtest / Agent / Scheduler / Validation / ApiRouting / Internal) |
| `models` | 数据模型入口，导出 `analysis_report` / `decision_action` / `stock` / `db` |
| `utils` | 工具函数 |

全局配置系统：

```rust
set_global_config(config: AppConfig)
get_global_config() -> DsaResult<AppConfig>
set_config_path(path: &str)
get_config_path() -> String
```

数据库实体 (29 个)：

| 分类 | 实体 |
|------|------|
| 行情 | `stock_daily` |
| 分析 | `analysis_history`, `fundamental_snapshot` |
| 决策 | `decision_signal`, `decision_signal_feedback`, `decision_signal_outcome` |
| 回测 | `backtest_result`, `backtest_summary` |
| Agent | `conversation_message`, `conversation_summary`, `agent_provider_turn` |
| 告警 | `alert_rule`, `alert_trigger`, `alert_notification`, `alert_cooldown` |
| 组合 | `portfolio_account`, `portfolio_position`, `portfolio_position_lot`, `portfolio_trade`, `portfolio_cash_ledger`, `portfolio_daily_snapshot`, `portfolio_corporate_action`, `portfolio_fx_rate` |
| 情报 | `intelligence_item`, `intelligence_source`, `news_intel` |
| 系统 | `llm_usage`, `watchlist_stock`, `schema_migration` |

Feature flags: `mysql` (默认) / `sqlite`

---

### dsa-pipeline — 分析管道层

| 模块 | 职责 |
|------|------|
| `technical` | 技术指标计算 (MA / MACD / KDJ / RSI / BOLL) |
| `pipeline` | 分析管道编排：获取行情 → 计算指标 → 构建 Prompt → 调用 LLM → 渲染报告 |
| `context_builder` | 从行情数据组装 LLM 上下文 |
| `prompt_templates` | Prompt 模板管理 |
| `market_review` | 大盘综述生成 (板块涨跌 / 资金流向) |
| `report_renderer` | 报告渲染与格式化 |

---

### dsa-agent — 多 Agent 层

#### Agent 体系

```
orchestrator (standard / adaptive)
├── skill_agent          # 技能路由，分发到专业 Agent
├── technical_agent      # 技术分析专用
├── decision_agent       # 决策建议专用
├── risk_agent           # 风险评估专用
├── strategy_agent       # 策略生成专用
├── portfolio_agent      # 组合管理专用
├── intel_agent          # 情报分析专用
└── react_executor       # ReAct 循环 (Thought → Action → Observation)
```

#### 工具体系 (9 类)

| 工具文件 | 功能 |
|----------|------|
| `data_tools` | 行情数据查询 |
| `analysis_tools` | 技术指标计算 |
| `market_tools` | 市场数据 |
| `backtest_tools` | 回测执行 |
| `chip_tools` | 筹码分布 |
| `pattern_tools` | 形态识别 |
| `portfolio_tools` | 组合查询 |
| `history_tools` | 历史数据 |
| `search_tools` | 网络搜索 |

#### 其他模块

| 模块 | 职责 |
|------|------|
| `conversation` | 对话管理、会话创建 / 恢复 |
| `memory` | Agent 记忆系统 |
| `skills/router` | 技能路由逻辑 |
| `skills/defaults` | 默认技能定义 |
| `tools/registry` | 工具注册系统 |

---

### dsa-backtest — 回测层

| 模块 | 职责 |
|------|------|
| `engine` | 回测执行引擎：基于历史分析信号评估收益 |
| `signal` | 信号追踪与评估：胜率 / 方向准确率 / 平均收益 / 止损触发率 |
| `report` | 回测报告生成 |

可配置参数：
- `eval_window_days` — 评估窗口天数
- `min_age_days` — 最小信号年龄
- `neutral_band_pct` — 中性带百分比 (小幅波动视为持平)

---

### dsa-service — 业务服务层

23 个服务模块，每个实现 `dispatch(&self, method: &str, params: &Value) -> DsaResult<Value>`：

| 服务 | 职责 |
|------|------|
| `stock` | 行情数据 (报价 / 历史 / 实时 / 搜索 / 关注列表) |
| `analysis` | 个股分析 (分析 / 报告 / 列表 / 最新) |
| `market` | 市场数据 (概览 / 板块 / 指数 / 综述) |
| `market_context` | 市场上下文 (获取 / 刷新) |
| `agent` | Agent 对话 (chat / stream / history / sessions) |
| `backtest` | 策略回测 (run / list / detail / compare) |
| `backtest_worker` | 回测任务 (queue / status / cancel) |
| `scheduler` | 定时调度 (status / trigger / config) |
| `portfolio` | 组合管理 (create / list / detail / update / performance) |
| `screening` | 策略筛选 (screen / presets / save) |
| `decision` | 决策信号 (list / detail / stats) |
| `decision_extractor` | 决策提取 (extract / batch) |
| `intelligence` | 情报中心 (news / digest / search) |
| `alert` | 告警管理 (create / list / update / delete / trigger) |
| `alert_worker` | 告警任务 (check / status) |
| `notification` | 通知推送 (send / channels / test / route) |
| `search` | 搜索服务 (search / providers) |
| `social_sentiment` | 社交情绪 (analyze / trending) |
| `name_resolver` | 名称解析 (resolve / search) |
| `report` | 报告管理 (generate / list / detail / export) |
| `config` | 配置管理 (get / update / reload) |
| `usage` | 用量统计 (summary / detail / export) |
| `system` | 系统信息 (info / health / stats) |
| `indicator` | 指标数据 |
| `bot` | Bot 平台 (dispatcher / commands / platforms) |

通知渠道 (14 种)：钉钉 / 飞书 / 企业微信 / Telegram / Bark / Email / Discord / Slack / Pushover / PushPlus / ServerChan / ntfy / Gotify / 自定义 Webhook

Bot 平台：钉钉 (Webhook + 签名验证) / 飞书 (Webhook + 事件回调) / Discord (Webhook + 交互)

---

### dsa-server — 接口层

#### 路由模式

**模块路由分发** (主要)：
```
POST /api/v1/{module}/{method}
Content-Type: application/json

{"method": "...", "params": {...}}
```

**REST 接口** (认证)：
```
POST /api/v1/auth/login
POST /api/v1/auth/register
GET  /api/v1/auth/profile
POST /api/v1/auth/profile/update
POST /api/v1/auth/change-password
```

**SSE 流式接口**：
```
POST /api/v1/agent/chat/stream    # Agent 对话流
POST /api/v1/analysis/stream      # 分析流
```

**其他**：
```
GET  /health                       # 健康检查
POST /api/v1/proxy                 # 代理
```

#### 路由分发模块 (25 个)

stock / analysis / market / agent / backtest / scheduler / portfolio / config / decision / intelligence / alert / usage / system / screening / notification / search / social_sentiment / backtest_worker / alert_worker / decision_extractor / market_context / name_resolver / report / bot / indicator

---

### dsa-app — 桌面应用层

Tauri 2 桌面应用，架构：

```
dsa-app (Tauri Window)
├── 后端线程: Actix-web on 127.0.0.1:18080
│   ├── 静态文件服务 (web/dist/)
│   └── 完整 API 服务
├── WebView → http://127.0.0.1:18080
├── 系统托盘 (显示窗口 / 退出)
├── 插件: shell / dialog / updater / process
└── 自动更新 (check_update / install_update)
```

特性：
- 关闭窗口最小化到托盘 (非退出)
- 双击托盘图标恢复窗口
- Release 模式自动重定向 WebView 到内嵌服务
- 支持自动更新 (pubkey 验证)
- 打包: macOS dmg / Windows nsis

---

## 前端架构

### 目录结构

```
web/src/
├── api/            # API 调用封装 (18 模块)
│   ├── index.ts    # callApi() 统一入口，自动检测 Tauri/Standalone
│   ├── agent.ts    ├── alert.ts    ├── analysis.ts
│   ├── auth.ts     ├── backtest.ts ├── decision.ts
│   ├── indicator.ts ├── intelligence.ts ├── market.ts
│   ├── notification.ts ├── portfolio.ts ├── scheduler.ts
│   ├── screening.ts ├── search.ts  ├── stock.ts
│   ├── system.ts   └── usage.ts
├── assets/         # 静态资源
├── components/     # 通用组件
│   └── common/
│       ├── KlineChart.vue        # K 线图 (ECharts)
│       ├── MarkdownRenderer.vue  # Markdown 渲染
│       ├── ScoreGauge.vue        # 情绪仪表盘
│       └── StockAutocomplete.vue # 股票搜索自动补全
├── composables/    # 组合式函数
│   ├── useTradingInterval.ts     # 交易时段判断
│   └── useUpdater.ts             # Tauri 自动更新
├── layout/         # 布局组件
│   ├── AppHeader.vue             # 顶部导航
│   ├── AppLayout.vue             # 主布局
│   └── SidebarNav.vue            # 侧边栏
├── router/         # 路由配置
├── stores/         # Pinia 状态管理
│   ├── auth.ts     # 认证状态 + Token
│   ├── app.ts      # 全局状态
│   ├── analysis.ts # 分析数据
│   └── chat.ts     # 对话状态
├── styles/         # 全局样式
│   └── global.scss
└── views/          # 页面视图 (12 页)
```

### API 调用机制

前端 API 客户端自动检测运行模式：

```typescript
// Tauri 模式: API base = http://127.0.0.1:18080
// Standalone 模式: API base = /api/v1 (由 Vite 代理)
```

模块路由统一调用：

```typescript
callApi(module, method, params)
// → POST /{module}/{method}  body: {method, params}
```

认证 Token 通过 Axios 拦截器自动注入。
