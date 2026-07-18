# 支持 make git <commit message> 直接传入提交信息
ifneq ($(filter git,$(MAKECMDGOALS)),)
  GIT_MSG_ARGS := $(wordlist 2,$(words $(MAKECMDGOALS)),$(MAKECMDGOALS))
  $(foreach _g,$(GIT_MSG_ARGS),$(eval $(_g):;@:))
endif

.PHONY: dev dev-server dev-web build build-server build-web clean check install run stop env db-init git help tauri-dev tauri-build tauri-release bump-version show-version

# 项目配置
CONFIG     ?= conf/config.toml
BIN        := target/debug/dsa
PORT       ?= 8000
WEB_PORT   ?= 5173

# 数据库默认配置 (可通过环境变量覆盖)
DB_HOST    ?= 127.0.0.1
DB_PORT    ?= 3306
DB_NAME    ?= dsa
DB_USER    ?= root
DB_PASS    ?=

# 颜色输出
CYAN  := \033[36m
GREEN := \033[32m
YELLOW := \033[33m
RED   := \033[31m
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
	@echo "$(GREEN)数据库命令:$(NC)"
	@echo "  make env          创建 .env 环境变量文件"
	@echo "  make db-init      初始化数据库 (创建库 + 启动迁移)"
	@echo ""
	@echo "$(GREEN)运维命令:$(NC)"
	@echo "  make run          运行后端 (需先 build)"
	@echo "  make stop         停止运行中的服务"
	@echo "  make check        类型检查 (cargo check + vue-tsc)"
	@echo "  make clean        清理构建产物"
	@echo "  make install      安装前端依赖"
	@echo ""
	@echo "$(GREEN)Git 命令:$(NC)"
	@echo "  make git          提交并推送 (交互输入 message)"
	@echo "  make git msg      提交并推送 (直接传入 message)"
	@echo "  make git MSG=xxx  提交并推送 (通过变量传入 message)"
	@echo ""
	@echo "$(GREEN)Tauri 桌面应用:$(NC)"
	@echo "  make tauri-dev    启动 Tauri 开发模式 (桌面窗口)"
	@echo "  make tauri-build  构建 Tauri 桌面应用 (Debug)"
	@echo "  make tauri-release 构建 Tauri 桌面应用 (Release)"
	@echo ""
	@echo "$(GREEN)环境变量:$(NC)"
	@echo "  CONFIG=$(CONFIG)     配置文件路径"
	@echo "  DB_HOST=$(DB_HOST)   数据库主机"
	@echo "  DB_PORT=$(DB_PORT)   数据库端口"
	@echo "  DB_NAME=$(DB_NAME)   数据库名称"
	@echo "  DB_USER=$(DB_USER)   数据库用户"
	@echo "  DB_PASS=***          数据库密码"

# ============================================================
# 环境准备
# ============================================================

env: ## 创建 .env 环境变量文件
	@if [ ! -f .env ]; then \
		echo "$(GREEN)Creating .env file...$(NC)"; \
		echo "# DSA 环境变量配置" > .env; \
		echo "# 数据库密码" >> .env; \
		echo "DSA_DB_PASSWORD=$(DB_PASS)" >> .env; \
		echo "" >> .env; \
		echo "# LLM API 密钥" >> .env; \
		echo "DEEPSEEK_API_KEY=" >> .env; \
		echo "" >> .env; \
		echo "# 搜索 API 密钥" >> .env; \
		echo "SERPER_API_KEY=" >> .env; \
		echo "BING_SEARCH_API_KEY=" >> .env; \
		echo "GOOGLE_SEARCH_API_KEY=" >> .env; \
		echo "$(GREEN).env created. Please fill in your API keys and database password.$(NC)"; \
	else \
		echo "$(YELLOW).env already exists, skipping.$(NC)"; \
	fi

db-init: env ## 初始化数据库 (创建 dsa 库)
	@echo "$(GREEN)[Database] Creating database $(DB_NAME) if not exists...$(NC)"
	@if command -v mysql > /dev/null 2>&1; then \
		mysql -h $(DB_HOST) -P $(DB_PORT) -u $(DB_USER) \
			$$( [ -n "$(DB_PASS)" ] && echo "-p$(DB_PASS)" || echo "" ) \
			-e "CREATE DATABASE IF NOT EXISTS \`$(DB_NAME)\` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;" 2>/dev/null \
		&& echo "$(GREEN)Database $(DB_NAME) created/verified.$(NC)" \
		|| echo "$(RED)Failed to connect MySQL. Please check database config in conf/config.toml$(NC)"; \
	elif command -v docker > /dev/null 2>&1; then \
		echo "$(YELLOW)mysql client not found, trying docker...$(NC)"; \
		docker exec $$(docker ps --filter "ancestor=mysql" -q | head -1) \
			mysql -u $(DB_USER) \
			$$( [ -n "$(DB_PASS)" ] && echo "-p$(DB_PASS)" || echo "" ) \
			-e "CREATE DATABASE IF NOT EXISTS \`$(DB_NAME)\` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;" 2>/dev/null \
		&& echo "$(GREEN)Database $(DB_NAME) created/verified via docker.$(NC)" \
		|| echo "$(RED)No MySQL available. Please start MySQL first.$(NC)"; \
	else \
		echo "$(RED)No MySQL client or Docker found. Please install MySQL or start a Docker container.$(NC)"; \
		echo "$(YELLOW)Hint: docker run -d --name mysql-dsa -e MYSQL_ROOT_PASSWORD=root -e MYSQL_DATABASE=dsa -p 3306:3306 mysql:8$(NC)"; \
	fi

# ============================================================
# 开发模式
# ============================================================

dev: env build-server ## 启动开发环境 (后端 + 前端)
	@echo "$(CYAN)Starting DSA dev environment...$(NC)"
	@trap '$(MAKE) stop; exit 0' INT TERM; \
	$(MAKE) dev-server & \
	$(MAKE) dev-web & \
	wait

dev-server: ## 启动后端开发服务 (自动重编译)
	@echo "$(GREEN)[Backend] Starting on port $(PORT)...$(NC)"
	DSA_DEV=1 cargo run --bin dsa -- --config $(CONFIG)

dev-web: install ## 启动前端开发服务器
	@echo "$(GREEN)[Frontend] Starting on port $(WEB_PORT)...$(NC)"
	cd web && DSA_STANDALONE=1 DSA_DEV=1 npm run dev

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

# ============================================================
# Git
# ============================================================

git: ## 提交并推送代码 (make git <message> 或 make git MSG=<message>)
	@set -e; \
	msg=''; \
	if [ -n "$(strip $(MSG))" ]; then \
		msg='$(subst ','\'\'\'',$(MSG))'; \
	elif [ -n "$(strip $(GIT_MSG_ARGS))" ]; then \
		msg='$(subst ','\'\'\'',$(GIT_MSG_ARGS))'; \
	else \
		printf 'input commit message: '; read -r msg; \
	fi; \
	git add . && \
	git commit -a -m "$$msg" && \
	git pull && \
	git push && \
	echo "$(GREEN)git commit and push success$(NC)"

# ============================================================
# 版本管理
# ============================================================

VERSION_FILE := .version

bump-version: ## 自动递增补丁版本号
	@if [ ! -f $(VERSION_FILE) ]; then echo "0.1.0" > $(VERSION_FILE); fi
	@cur=$$(cat $(VERSION_FILE)); \
	major=$$(echo $$cur | cut -d. -f1); \
	minor=$$(echo $$cur | cut -d. -f2); \
	patch=$$(echo $$cur | cut -d. -f3); \
	new_patch=$$((patch + 1)); \
	new_ver="$$major.$$minor.$$new_patch"; \
	echo "$$new_ver" > $(VERSION_FILE); \
	sed -i '' 's/^version = ".*"/version = "'$$new_ver'"/' Cargo.toml; \
	sed -i '' 's/"version": ".*"/"version": "'$$new_ver'"/' crates/dsa-app/tauri.conf.json; \
	sed -i '' 's/"version": ".*"/"version": "'$$new_ver'"/' web/package.json; \
	echo "$(GREEN)Version bumped: $$cur -> $$new_ver$(NC)"

show-version: ## 显示当前版本号
	@if [ -f $(VERSION_FILE) ]; then cat $(VERSION_FILE); else echo "0.1.0"; fi

tauri-dev: install ## 启动 Tauri 桌面应用开发模式
	@echo "$(CYAN)Starting DSA Tauri dev...$(NC)"
	cd crates/dsa-app && DSA_API_PORT=18080 TAURI_DEV=1 npx --prefix ../../web tauri dev

tauri-build: bump-version build-web ## 构建 Tauri 桌面应用 (自动递增版本号)
	@echo "$(GREEN)[Tauri] Building desktop app v$$(cat $(VERSION_FILE))...$(NC)"
	cd crates/dsa-app && cargo tauri build

tauri-release: tauri-build ## 构建 Tauri 桌面应用 (Release，等同 tauri-build)
