.PHONY: dev dev-server dev-web build build-server build-web clean check install run stop help

# 项目配置
CONFIG     ?= conf/config.toml
BIN        := target/debug/dsa
PORT       ?= 8000
WEB_PORT   ?= 5173

# 颜色输出
CYAN  := \033[36m
GREEN := \033[32m
YELLOW := \033[33m
NC    := \033[0m

help: ## 显示帮助信息
	@echo "$(CYAN)DSA - Daily Stock Analysis$(NC)"
	@echo ""
	@echo "$(GREEN)开发命令:$(NC)"
	@echo "  make dev          启动开发环境 (后端 + 前端 concurrently)"
	@echo "  make dev-server   仅启动后端服务"
	@echo "  make dev-web      仅启动前端开发服务器"
	@echo ""
	@echo "$(GREEN)构建命令:$(NC)"
	@echo "  make build        构建后端 + 前端"
	@echo "  make build-server 仅构建后端"
	@echo "  make build-web    仅构建前端"
	@echo "  make release      Release 模式构建后端"
	@echo ""
	@echo "$(GREEN)运维命令:$(NC)"
	@echo "  make run          运行后端 (需先 build)"
	@echo "  make stop         停止运行中的服务"
	@echo "  make check        类型检查 (cargo check + vue-tsc)"
	@echo "  make clean        清理构建产物"
	@echo "  make install      安装前端依赖"
	@echo ""
	@echo "$(GREEN)环境变量:$(NC)"
	@echo "  CONFIG=$(CONFIG)  配置文件路径"
	@echo "  PORT=$(PORT)      后端端口"
	@echo "  WEB_PORT=$(WEB_PORT) 前端端口"

# ============================================================
# 开发模式
# ============================================================

dev: build-server ## 启动开发环境 (后端 + 前端)
	@echo "$(CYAN)Starting DSA dev environment...$(NC)"
	@trap 'kill 0; exit 0' INT TERM; \
	$(MAKE) dev-server & \
	$(MAKE) dev-web & \
	wait

dev-server: ## 启动后端开发服务 (自动重编译)
	@echo "$(GREEN)[Backend] Starting on port $(PORT)...$(NC)"
	cargo run --bin dsa -- --config $(CONFIG)

dev-web: install ## 启动前端开发服务器
	@echo "$(GREEN)[Frontend] Starting on port $(WEB_PORT)...$(NC)"
	cd web && npm run dev

# ============================================================
# 构建
# ============================================================

build: build-server build-web ## 构建后端 + 前端

build-server: ## 构建后端
	@echo "$(GREEN)[Backend] Building...$(NC)"
	cargo build --bin dsa

build-web: install ## 构建前端
	@echo "$(GREEN)[Frontend] Building...$(NC)"
	cd web && npm run build

release: ## Release 模式构建后端
	@echo "$(GREEN)[Backend] Building release...$(NC)"
	cargo build --release --bin dsa

# ============================================================
# 运行
# ============================================================

run: build ## 运行后端服务 (需先构建)
	@echo "$(GREEN)[Backend] Running on port $(PORT)...$(NC)"
	./$(BIN) --config $(CONFIG)

stop: ## 停止运行中的 DSA 服务
	@echo "$(YELLOW)Stopping DSA services...$(NC)"
	@pkill -f "target/debug/dsa" 2>/dev/null || true
	@pkill -f "vite.*dsa-web" 2>/dev/null || true
	@echo "$(GREEN)Stopped.$(NC)"

# ============================================================
# 检查 & 清理
# ============================================================

check: ## 类型检查
	@echo "$(GREEN)[Backend] cargo check...$(NC)"
	cargo check
	@echo "$(GREEN)[Frontend] vue-tsc...$(NC)"
	cd web && npx vue-tsc --noEmit

clean: ## 清理构建产物
	@echo "$(YELLOW)Cleaning...$(NC)"
	cargo clean
	rm -rf web/dist
	@echo "$(GREEN)Cleaned.$(NC)"

install: ## 安装前端依赖 (仅在 node_modules 不存在时)
	@if [ ! -d web/node_modules ]; then \
		echo "$(GREEN)[Frontend] Installing dependencies...$(NC)"; \
		cd web && npm install; \
	fi
