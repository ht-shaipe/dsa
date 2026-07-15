# DSA API 接口参考

## 接口规范

### 模块路由分发

大部分 API 采用模块路由分发模式，统一入口格式：

```
POST /api/v1/{module}/{method}
Content-Type: application/json

{
  "method": "list",
  "params": { ... }
}
```

### REST 接口

认证相关接口使用标准 REST 风格。

### SSE 流式接口

Agent 对话和分析报告支持 SSE (Server-Sent Events) 流式响应。

---

## 认证接口

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/v1/auth/login` | 登录 |
| POST | `/api/v1/auth/register` | 注册 |
| GET | `/api/v1/auth/profile` | 获取用户信息 |
| POST | `/api/v1/auth/profile/update` | 更新用户信息 |
| POST | `/api/v1/auth/change-password` | 修改密码 |

---

## 系统接口

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/health` | 健康检查 |

---

## 业务模块接口

### stock — 股票数据

| method | 说明 |
|--------|------|
| `quote` | 获取股票报价 |
| `history` | 获取历史行情 |
| `realtime` | 获取实时行情 |
| `search` | 搜索股票 |
| `watchlist` | 关注列表操作 |

### analysis — 个股分析

| method | 说明 |
|--------|------|
| `analyze` | 触发个股分析 |
| `report` | 获取分析报告 |
| `list` | 分析列表 |
| `latest` | 最新分析 |

### market — 市场数据

| method | 说明 |
|--------|------|
| `overview` | 市场概览 |
| `sectors` | 板块数据 |
| `indices` | 指数数据 |
| `review` | 大盘综述 |

### market_context — 市场上下文

| method | 说明 |
|--------|------|
| `get` | 获取市场上下文 |
| `refresh` | 刷新市场上下文 |

### agent — Agent 对话

| method | 说明 |
|--------|------|
| `chat` | 发送对话 |
| `history` | 对话历史 |
| `sessions` | 会话列表 |

**流式接口**: `POST /api/v1/agent/chat/stream` (SSE)

### backtest — 策略回测

| method | 说明 |
|--------|------|
| `run` | 执行回测 |
| `list` | 回测列表 |
| `detail` | 回测详情 |
| `compare` | 回测对比 |

### backtest_worker — 回测任务

| method | 说明 |
|--------|------|
| `queue` | 加入队列 |
| `status` | 任务状态 |
| `cancel` | 取消任务 |

### scheduler — 定时调度

| method | 说明 |
|--------|------|
| `status` | 调度状态 |
| `trigger` | 手动触发 |
| `config` | 调度配置 |

### portfolio — 组合管理

| method | 说明 |
|--------|------|
| `create` | 创建账户 |
| `list` | 账户列表 |
| `detail` | 账户详情 |
| `update` | 更新账户 |
| `performance` | 绩效评估 |

### screening — 策略筛选

| method | 说明 |
|--------|------|
| `screen` | 执行筛选 |
| `presets` | 预设策略 |
| `save` | 保存策略 |

### decision — 决策信号

| method | 说明 |
|--------|------|
| `list` | 信号列表 |
| `detail` | 信号详情 |
| `stats` | 信号统计 |

### decision_extractor — 决策提取

| method | 说明 |
|--------|------|
| `extract` | 提取信号 |
| `batch` | 批量提取 |

### intelligence — 情报中心

| method | 说明 |
|--------|------|
| `news` | 新闻数据 |
| `digest` | 摘要 |
| `search` | 情报搜索 |

### alert — 告警管理

| method | 说明 |
|--------|------|
| `create` | 创建告警 |
| `list` | 告警列表 |
| `update` | 更新告警 |
| `delete` | 删除告警 |
| `trigger` | 触发记录 |

### alert_worker — 告警任务

| method | 说明 |
|--------|------|
| `check` | 检查告警 |
| `status` | 任务状态 |

### notification — 通知推送

| method | 说明 |
|--------|------|
| `send` | 发送通知 |
| `channels` | 渠道列表 |
| `test` | 测试渠道 |
| `route` | 路由规则 |

### search — 搜索服务

| method | 说明 |
|--------|------|
| `search` | 执行搜索 |
| `providers` | 搜索引擎列表 |

### social_sentiment — 社交情绪

| method | 说明 |
|--------|------|
| `analyze` | 情绪分析 |
| `trending` | 热门话题 |

### name_resolver — 名称解析

| method | 说明 |
|--------|------|
| `resolve` | 解析名称 |
| `search` | 搜索 |

### report — 报告管理

| method | 说明 |
|--------|------|
| `generate` | 生成报告 |
| `list` | 报告列表 |
| `detail` | 报告详情 |
| `export` | 导出报告 |

### config — 配置管理

| method | 说明 |
|--------|------|
| `get` | 获取配置 |
| `update` | 更新配置 |
| `reload` | 重载配置 |

### usage — 用量统计

| method | 说明 |
|--------|------|
| `summary` | 用量汇总 |
| `detail` | 用量明细 |
| `export` | 导出数据 |

### system — 系统信息

| method | 说明 |
|--------|------|
| `info` | 系统信息 |
| `health` | 健康状态 |
| `stats` | 系统统计 |

### indicator — 指标数据

| method | 说明 |
|--------|------|
| (dispatch) | 技术指标查询与计算 |

### bot — Bot 平台

| method | 说明 |
|--------|------|
| `dingtalk` | 钉钉 Bot |
| `feishu` | 飞书 Bot |
| `discord` | Discord Bot |
| `webhook` | 通用 Webhook |

---

## 通知渠道

| 渠道 | 配置字段 | 说明 |
|------|----------|------|
| 钉钉 | `dingtalk_webhook` | Webhook 机器人 |
| 飞书 | `feishu_webhook` | Webhook 机器人 |
| 企业微信 | `wecom_webhook` | Webhook 机器人 |
| Telegram | `telegram_bot_token` + `telegram_chat_id` | Bot API |
| Bark | `bark_url` | iOS 推送 |
| Email | `email_smtp_host/port/user/pass/from/to` | SMTP 邮件 |
| Discord | - | Webhook |
| Slack | - | Webhook |
| Pushover | - | 推送服务 |
| PushPlus | - | 微信推送 |
| ServerChan | - | Server 酱 |
| ntfy | - | 开源推送 |
| Gotify | - | 自建推送 |
| 自定义 Webhook | - | 通用 HTTP |

路由规则: **critical** → 全部已配置渠道, **warning** → 即时通讯 + 邮件, **info** → 仅日志

---

## SSE 流式接口

### Agent 对话流

```
POST /api/v1/agent/chat/stream
Content-Type: application/json

{
  "session_id": "xxx",
  "message": "分析贵州茅台近期走势",
  "skill": "technical"
}
```

响应: `Content-Type: text/event-stream`

```
data: {"type": "thought", "content": "需要查询茅台的行情数据..."}
data: {"type": "action", "tool": "data_tools", "input": {...}}
data: {"type": "observation", "content": "..."}
data: {"type": "answer", "content": "根据分析..."}
data: [DONE]
```

### 分析流

```
POST /api/v1/analysis/stream
Content-Type: application/json

{
  "code": "600519",
  "name": "贵州茅台"
}
```

响应: `Content-Type: text/event-stream`，逐步返回分析结果。
