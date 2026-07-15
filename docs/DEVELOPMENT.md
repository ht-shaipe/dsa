# DSA 开发指南

## 项目结构

```
dsa/
├── Cargo.toml          # workspace 根配置
├── Makefile            # 开发命令快捷入口
├── conf/config.toml    # 运行时配置
├── crates/
│   ├── dsa-core/       # 基础层
│   ├── dsa-pipeline/   # 管道层
│   ├── dsa-agent/      # Agent 层
│   ├── dsa-backtest/   # 回测层
│   ├── dsa-service/    # 业务层
│   ├── dsa-server/     # 接口层
│   └── dsa-app/        # 桌面层 (Tauri)
└── web/                # 前端
```

---

## 常用命令

```bash
# Makefile 快捷命令
make dev          # 启动开发环境 (后端 + 前端)
make dev-server   # 仅启动后端
make dev-web      # 仅启动前端
make build        # 构建后端 + 前端
make release      # Release 模式构建后端
make check        # 类型检查 (cargo check + vue-tsc)
make clean        # 清理构建产物
make env          # 生成 .env 文件
make db-init      # 初始化数据库
make git <msg>    # 提交并推送

# Tauri 桌面应用
make tauri-dev     # 启动 Tauri 开发模式
make tauri-release # 构建 Tauri Release

# Cargo 直接命令
cargo build                   # 编译
cargo run --bin dsa           # 运行后端
cargo run --bin dsa -- --config conf/config.local.toml  # 指定配置
cargo test                    # 运行测试
```

---

## 添加新 Service

1. 在 `crates/dsa-service/src/` 下创建 `xxx.rs` (或 `xxx/` 目录)

2. 实现 Service 结构体：

```rust
use dsa_core::errors::{DsaResult, DsaError};
use serde_json::Value;

pub struct XxxService;

impl XxxService {
    pub fn new() -> Self { Self }

    pub fn dispatch(&self, method: &str, params: &Value) -> DsaResult<Value> {
        match method {
            "list" => self.list(params),
            "create" => self.create(params),
            _ => Err(DsaError::ApiRouting(format!("unknown method: {method}"))),
        }
    }

    fn list(&self, _params: &Value) -> DsaResult<Value> {
        // ...
        Ok(serde_json::json!({}))
    }

    fn create(&self, _params: &Value) -> DsaResult<Value> {
        // ...
        Ok(serde_json::json!({}))
    }
}
```

3. 在 `crates/dsa-service/src/lib.rs` 中注册模块：

```rust
mod xxx;
pub use xxx::XxxService;
```

---

## 添加新 API 模块

1. 在 `crates/dsa-server/src/handler/` 下创建 `xxx.rs`

2. 实现 handler 函数：

```rust
use dsa_core::errors::DsaResult;
use dsa_core::Value;
use dsa_service::XxxService;

pub fn distribute(method: &str, params: &Value) -> DsaResult<Value> {
    let service = XxxService::new();
    service.dispatch(method, params)
}
```

3. 在 `crates/dsa-server/src/handler/mod.rs` 中声明模块：

```rust
pub mod xxx;
```

4. 在 `crates/dsa-server/src/router.rs` 的 `api_handler` match 中添加路由：

```rust
"xxx" => handler::xxx::distribute(&method, &params),
```

---

## 添加前端页面

1. 在 `web/src/views/` 下创建 `XxxView.vue`

2. 在 `web/src/router/index.ts` 的 children 中添加路由：

```typescript
{
  path: '/xxx',
  name: 'Xxx',
  component: () => import('@/views/XxxView.vue')
}
```

3. 如需 API 调用，在 `web/src/api/` 下创建 `xxx.ts`：

```typescript
import { callApi } from './index'

export const xxxApi = {
  list: (params?: any) => callApi('xxx', 'list', params),
  create: (params: any) => callApi('xxx', 'create', params),
}
```

4. 如需状态管理，在 `web/src/stores/` 下创建 `xxx.ts`

---

## 添加 Agent 工具

1. 在 `crates/dsa-agent/src/tools/` 下创建 `xxx_tools.rs`

2. 实现工具函数，注册到 `register_all.rs`

3. 如需新 Agent，在 `crates/dsa-agent/src/agents/` 下添加，并在 `orchestrator.rs` 中接入

---

## 添加通知渠道

1. 在 `crates/dsa-service/src/notification.rs` 的 `send_to_channel` 中添加新渠道实现

2. 在 `crates/dsa-core/src/config.rs` 的 `NotificationConfig` 中添加配置字段

3. 更新 `conf/config.toml` 模板

---

## 添加数据库实体

1. 在 `crates/dsa-core/src/models/db/` 下创建 `xxx.rs`

2. 使用 `deck` derive 宏定义实体：

```rust
use deck_model::entity;

#[entity(table = "xxx")]
pub struct Xxx {
    pub id: u64,
    pub name: String,
    // ...
}
```

3. 在 `crates/dsa-core/src/models/db/mod.rs` 中声明模块

4. 在 `crates/dsa-core/src/db/migration.rs` 中添加迁移

5. 重启后端，ORM 会自动执行迁移

---

## 代码规范

### Rust

- Edition 2021, Rust 1.75+
- 错误处理使用 `DsaResult<T>` (自定义 Result 类型)
- Service 层统一 `dispatch(method, params) -> DsaResult<Value>` 接口
- 序列化使用 `serde` + `serde_json`
- 配置使用 `toml` 格式

### TypeScript / Vue

- Vue 3 Composition API + `<script setup>`
- TypeScript 严格模式
- Element Plus 组件库
- 状态管理使用 Pinia
- API 调用统一走 `callApi()` 入口

### 数据库

- 使用 deck ORM (derive 宏)
- MySQL / SQLite 双数据库支持 (Feature flag)
- 自动迁移，无需手动建表
- 不要直接写 SQL，优先使用 ORM

---

## 依赖说明

本项目依赖多个本地 kit 库，路径在 `Cargo.toml` 中以绝对路径指定：

| Kit | 路径 | 用途 |
|-----|------|------|
| tube | `/Users/shaipe/workspace/rust/kit/tube` | 通用工具库 (derive / utils / fs / cmd / net / web / value / jwt) |
| qta | `/Users/shaipe/workspace/rust/kit/qta` | 行情数据 (crawler / tdx / sdk) |
| ai-llm-kit | `/Users/shaipe/workspace/rust/kit/ai/llm` | LLM 调用封装 |
| deck | `/Users/shaipe/workspace/rust/kit/deck` | ORM (MySQL / SQLite) |

> **注意**: 这些是本地路径依赖，克隆项目后需确保这些 kit 库也在对应路径下，或修改 `Cargo.toml` 中的路径。
