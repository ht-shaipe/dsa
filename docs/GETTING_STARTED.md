# DSA 使用指南

从零开始搭建和使用 DSA (Daily Stock Analysis) AI 驱动的每日股票分析系统。

---

## 第一步：环境准备

### 必需环境

| 工具 | 版本要求 | 用途 |
|------|----------|------|
| Rust | 1.75+ | 编译后端 |
| Node.js | 18+ | 前端开发/构建 |
| MySQL / MariaDB | 8.0+ / 10.5+ | 数据存储 |

### 可选环境

| 工具 | 用途 |
|------|------|
| Docker | 运行 MariaDB 容器 |
| tushare Token | 扩展行情数据源 |

---

## 第二步：安装依赖

```bash
# 克隆项目
git clone <repo-url> && cd dsa

# 编译后端（首次编译较慢，约 2~5 分钟）
cargo build --release

# 安装前端依赖
cd web && npm install && cd ..
```

---

## 第三步：准备数据库

### 方式 A：使用 Docker（推荐）

```bash
docker run -d \
  --name mariadb \
  -p 3306:3306 \
  -e MYSQL_ROOT_PASSWORD=mysql.8889_root \
  -e MYSQL_DATABASE=dsa \
  mariadb:10.11
```

### 方式 B：使用已有 MySQL

手动创建数据库：

```sql
CREATE DATABASE IF NOT EXISTS dsa DEFAULT CHARSET utf8mb4;
```

> 数据表会在首次启动时由 deck ORM 自动创建，无需手动建表。

---

## 第四步：配置系统

编辑 `conf/config.toml`，按需修改以下关键配置：

### 4.1 数据库连接

```toml
[database]
host = "127.0.0.1"
port = 3306
name = "dsa"
user = "root"
password = "your_password"
# 建议使用环境变量代替明文密码
# password_env = "DSA_DB_PASSWORD"
```

### 4.2 LLM 大模型（必须配置，否则 AI 分析不可用）

```toml
[llm]
provider = "deepseek"
model = "deepseek-chat"
# API 密钥从环境变量读取
api_key_env = "DEEPSEEK_API_KEY"
temperature = 0.7
timeout_seconds = 60
```

使用前设置环境变量：

```bash
export DEEPSEEK_API_KEY="sk-your-api-key"
```

**支持的大模型提供商**：

| provider | model 示例 | 说明 |
|----------|-----------|------|
| deepseek | deepseek-chat | DeepSeek 官方 API |
| openai | gpt-4o / gpt-4o-mini | OpenAI API |
| anthropic | claude-3-5-sonnet | Anthropic API |

### 4.3 服务端口

```toml
[server]
host = "0.0.0.0"
port = 8000
```

### 4.4 认证密码（可选）

默认无需登录。如需开启认证：

```toml
[server]
auth_password = "your_password"
# 或从环境变量读取
# auth_password_env = "DSA_PASSWORD"
```

### 4.5 关注列表（可选）

```toml
[stock]
watchlist = ["600519", "300750", "002594"]
enable_realtime = true
```

---

## 第五步：启动系统

### 5.1 启动后端

```bash
# 开发模式
cargo run

# 或使用编译好的二进制
./target/release/dsa

# 指定配置文件
./target/release/dsa --config conf/config.local.toml
```

启动成功后会看到：

```
[2026-07-07 11:00:00] MySQL连接池已注册: mysql://root@127.0.0.1:3306/dsa
[2026-07-07 11:00:00] DSA server starting at 0.0.0.0:8000
[2026-07-07 11:00:00] LLM provider: deepseek, model: deepseek-chat
```

### 5.2 启动前端开发服务器

```bash
cd web && npm run dev
```

前端会在 `http://localhost:5173` 启动，自动代理 `/api` 请求到后端 8000 端口。

### 5.3 生产部署（可选）

```bash
# 构建前端静态文件
cd web && npm run build

# 后端会自动托管 web/dist/ 目录下的静态文件
# 直接访问 http://your-server:8000 即可
./target/release/dsa
```

---

## 第六步：开始使用

浏览器打开 `http://localhost:5173`，进入系统。

### 6.1 工作台（首页）

- 查看**大盘指数**概览（上证、深证、创业板）
- 在搜索框输入**股票代码或名称**，选择股票后点击"分析"
- 系统调用 LLM 生成分析报告，包含：
  - 情绪评分（0~100 仪表盘）
  - 买入/卖出/持有建议
  - 理想买入价、止损价、目标价
  - 风险提示
  - Markdown 格式完整报告

### 6.2 Agent 问股（/chat）

- 与 AI Agent 多轮对话，支持 SSE 流式响应
- 内置专业 Agent：技术分析、决策建议、风险评估、策略生成、组合管理、情报分析
- Agent 可调用工具：行情查询、指标计算、回测执行、筹码分析、形态识别、网络搜索
- 选择技能/策略后发送问题，如："分析贵州茅台近期走势"或"当前市场有哪些风险"

### 6.3 选股筛选（/screening）

- 使用预设策略筛选股票（如放量突破、MACD金叉等）
- 探索市场热点主题
- 自定义多维度筛选条件

### 6.4 投资组合（/portfolio）

- 创建账户：设置名称、市场（A股/港股/美股）、券商、初始资金
- 记录交易：买入/卖出操作，自动计算持仓成本
- 查看持仓：实时市值、浮动盈亏、持仓占比
- 组合支持 FIFO 分批成本追踪和跨市场汇率换算

### 6.5 决策信号（/decision-signals）

- 系统从分析报告中自动提取买卖信号
- 信号类型：buy / add / hold / reduce / sell / watch / avoid
- 查看信号详情：入场价、止损价、目标价、推理依据
- 信号追踪：记录实际收益、最大回撤、方向准确性
- 用户反馈：对信号表示同意/不同意/部分同意

### 6.6 回测分析（/backtest）

- 对历史分析信号进行回测验证
- 评估指标：胜率、方向准确率、平均收益、止损/止盈触发率
- 可配置评估窗口天数和中性带

### 6.7 预警中心（/alerts）

- 创建告警规则：价格突破、涨跌幅超限、成交量异常
- 支持冷却时间和去重，避免重复推送
- 查看触发记录和通知历史

### 6.8 用量统计（/usage）

- LLM Token 消耗统计
- 费用估算
- 按模型/操作的分类明细

### 6.9 系统设置（/settings）

- 查看/修改运行时配置
- LLM 连接测试
- 模型发现
- 配置导入/导出

---

## 第七步：进阶配置

### 7.1 开启定时任务

定时任务会在指定时间自动执行分析（对关注列表中的股票生成报告）：

```toml
[scheduler]
enabled = true
run_immediately = true  # 启动后立即执行一次
times = ["18:00"]       # 每天 18:00 执行
```

### 7.2 开启 Agent 多智能体

```toml
[agent]
enabled = true
arch = "multi"           # multi=多Agent协作, single=单Agent
orchestrator_mode = "standard"
max_steps = 10           # 单次任务最大步数
```

### 7.3 配置通知推送

支持 14+ 种通知渠道。以钉钉为例：

```toml
[notification]
dingtalk_webhook = "https://oapi.dingtalk.com/robot/send?access_token=xxx"
```

以 Telegram 为例：

```toml
[notification]
telegram_bot_token = "123456:ABC-DEF"
telegram_chat_id = "123456789"
```

通知分级路由规则：
- **critical**（严重）→ 全部已配置渠道
- **warning**（警告）→ 即时通讯 + 邮件
- **info**（信息）→ 仅日志

### 7.4 配置搜索服务

为 Agent 提供网络搜索能力：

```toml
[search]
default_provider = "serper"
serper_api_key_env = "SERPER_API_KEY"
```

```bash
export SERPER_API_KEY="your-serper-key"
```

### 7.5 配置大盘综述

```toml
[market_review]
enabled = true
region = "cn"  # cn=A股, hk=港股, us=美股
```

### 7.6 敏感信息最佳实践

所有包含密钥/密码的配置项都支持 `_env` 后缀，从环境变量读取：

| 配置字段 | 环境变量字段 | 示例 |
|----------|-------------|------|
| `password` | `password_env` | `DSA_DB_PASSWORD` |
| `llm.api_key` (内联) | `llm.api_key_env` | `DEEPSEEK_API_KEY` |
| `email_pass` | `email_pass_env` | `DSA_EMAIL_PASS` |
| `serper_api_key` | `serper_api_key_env` | `SERPER_API_KEY` |

建议将密钥写入 `.env` 或系统环境变量，而非明文写在 config.toml 中。

---

## 第八步：常见问题

### Q: 启动后端报数据库连接失败

1. 确认 MySQL/MariaDB 正在运行：`docker ps` 或 `mysqladmin ping`
2. 确认 `conf/config.toml` 中数据库密码正确
3. 确认数据库 `dsa` 已创建
4. 如果使用 `password_env`，确认环境变量已设置

### Q: AI 分析返回错误

1. 确认 LLM API 密钥已设置：`echo $DEEPSEEK_API_KEY`
2. 在"系统设置"页面点击"测试 LLM 连接"验证
3. 检查网络是否能访问 LLM API（DeepSeek: api.deepseek.com）
4. 如果使用代理，配置 `conf/config.toml` 中的 `[proxy]` 段

### Q: 前端页面空白或接口报错

1. 确认后端在 8000 端口运行：`curl http://localhost:8000/health`
2. 确认前端 Vite 代理配置正确（`web/vite.config.ts` 中 target 为 `http://127.0.0.1:8000`）
3. 查看浏览器开发者工具 Network 面板，检查请求是否到达后端

### Q: 行情数据为空

1. 确认 `[stock] enable_realtime = true`
2. A 股行情数据来自东方财富/腾讯/新浪公开接口，需联网
3. 非交易时段部分数据可能为空

### Q: 如何更新数据库表结构

重启后端即可。deck ORM 会在启动时自动检测并执行缺失的迁移，不会删除已有数据。

---

## 快速检查清单

启动系统前，按此清单逐项确认：

- [ ] MySQL/MariaDB 运行中，`dsa` 数据库已创建
- [ ] `conf/config.toml` 数据库连接信息正确
- [ ] LLM API 密钥已设置（环境变量或配置文件）
- [ ] 后端启动成功，日志显示 `DSA server starting at 0.0.0.0:8000`
- [ ] `curl http://localhost:8000/health` 返回 `{"status":"ok"}`
- [ ] 前端 `npm run dev` 启动成功
- [ ] 浏览器访问 `http://localhost:5173` 可看到工作台页面
