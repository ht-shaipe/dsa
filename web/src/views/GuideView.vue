<template>
  <div class="guide-view">
    <el-card shadow="hover" style="margin-bottom: 20px">
      <template #header>
        <div style="display:flex;justify-content:space-between;align-items:center">
          <span style="font-size:18px;font-weight:600">DSA 使用手册</span>
          <el-tag>v0.1.0</el-tag>
        </div>
      </template>
      <el-alert type="info" :closable="false" show-icon style="margin-bottom:20px">
        DSA 是 AI 驱动的每日股票分析系统。请按下方步骤配置和使用。
      </el-alert>
    </el-card>

    <el-steps :active="activeStep" align-center style="margin-bottom:30px">
      <el-step title="环境准备" />
      <el-step title="配置系统" />
      <el-step title="开始使用" />
      <el-step title="进阶功能" />
    </el-steps>

    <div v-show="activeStep === 0">
      <el-card shadow="hover" style="margin-bottom:20px">
        <template #header>
          <div style="display:flex;align-items:center;gap:8px">
            <el-icon color="var(--el-color-primary)" :size="20"><Monitor /></el-icon>
            <span style="font-weight:600">第一步：环境准备</span>
          </div>
        </template>

        <el-descriptions :column="1" border>
          <el-descriptions-item label="后端运行">
            后端使用 Rust 编写，需先编译再启动。首次编译约 2~5 分钟。
            <div style="margin-top:8px">
              <code class="dsa-code">cargo build --release</code>
            </div>
          </el-descriptions-item>
          <el-descriptions-item label="前端运行">
            前端使用 Vue 3 + Vite，需安装依赖后启动开发服务器。
            <div style="margin-top:8px">
              <code class="dsa-code">cd web && npm install && npm run dev</code>
            </div>
          </el-descriptions-item>
          <el-descriptions-item label="数据库">
            需要 MySQL 8.0+ 或 MariaDB 10.5+，数据表会在后端首次启动时自动创建。
            <div style="margin-top:8px">
              <code class="dsa-code">docker run -d --name mariadb -p 3306:3306 -e MYSQL_ROOT_PASSWORD=your_pwd -e MYSQL_DATABASE=dsa mariadb:10.11</code>
            </div>
          </el-descriptions-item>
          <el-descriptions-item label="启动顺序">
            <el-timeline>
              <el-timeline-item>1. 启动数据库</el-timeline-item>
              <el-timeline-item>2. 启动后端 <el-tag size="small" type="info">端口 8000</el-tag></el-timeline-item>
              <el-timeline-item>3. 启动前端 <el-tag size="small" type="info">端口 5173</el-tag></el-timeline-item>
              <el-timeline-item>4. 浏览器打开 <el-link type="primary">http://localhost:5173</el-link></el-timeline-item>
            </el-timeline>
          </el-descriptions-item>
          <el-descriptions-item label="验证启动">
            后端启动成功后，访问 <el-link type="primary" href="/health" target="_blank">/health</el-link> 应返回
            <code class="dsa-code">{"status":"ok"}</code>
          </el-descriptions-item>
        </el-descriptions>
      </el-card>
    </div>

    <div v-show="activeStep === 1">
      <el-card shadow="hover" style="margin-bottom:20px">
        <template #header>
          <div style="display:flex;align-items:center;gap:8px">
            <el-icon color="var(--el-color-primary)" :size="20"><Setting /></el-icon>
            <span style="font-weight:600">第二步：配置系统</span>
          </div>
        </template>

        <el-alert type="warning" :closable="false" show-icon style="margin-bottom:16px">
          所有配置在「系统设置」页面中完成，也可直接编辑 <code class="dsa-code">conf/config.toml</code>
        </el-alert>

        <el-collapse v-model="configCollapse">
          <el-collapse-item title="LLM 大模型配置（必须）" name="llm">
            <p style="color:var(--el-text-color-secondary);margin-bottom:12px">
              AI 分析功能依赖大语言模型，必须先配置 API Key 才能使用。
            </p>
            <el-steps direction="vertical" :active="3" :space="60">
              <el-step title="前往「系统设置 → LLM配置」" />
              <el-step title="选择供应商（推荐 DeepSeek 或 OpenAI）" />
              <el-step title="填入 API Key" />
              <el-step title="点击「测试连接」确认可用" />
            </el-steps>
            <el-divider />
            <el-descriptions :column="2" border size="small">
              <el-descriptions-item label="DeepSeek">价格低、中文强，推荐入门使用</el-descriptions-item>
              <el-descriptions-item label="OpenAI">gpt-4o / gpt-4o-mini，综合能力强</el-descriptions-item>
              <el-descriptions-item label="Anthropic">claude-3-5-sonnet，推理能力突出</el-descriptions-item>
              <el-descriptions-item label="Ollama">本地部署，无需 API Key</el-descriptions-item>
            </el-descriptions>
          </el-collapse-item>

          <el-collapse-item title="数据库配置" name="db">
            <p style="color:var(--el-text-color-secondary);margin-bottom:8px">
              编辑 <code class="dsa-code">conf/config.toml</code> 中的 <code class="dsa-code">[database]</code> 段：
            </p>
            <el-descriptions :column="1" border size="small">
              <el-descriptions-item label="host">数据库地址，默认 127.0.0.1</el-descriptions-item>
              <el-descriptions-item label="port">端口号，默认 3306</el-descriptions-item>
              <el-descriptions-item label="name">数据库名，默认 dsa</el-descriptions-item>
              <el-descriptions-item label="user / password">用户名和密码</el-descriptions-item>
            </el-descriptions>
          </el-collapse-item>

          <el-collapse-item title="认证配置（可选）" name="auth">
            <p style="color:var(--el-text-color-secondary)">
              默认无需登录。如需开启密码保护，在 <code class="dsa-code">config.toml</code> 的 <code class="dsa-code">[server]</code> 中设置
              <code class="dsa-code">auth_password</code>，或在「系统设置 → 认证配置」中修改密码。
            </p>
          </el-collapse-item>

          <el-collapse-item title="通知推送（可选）" name="notif">
            <p style="color:var(--el-text-color-secondary);margin-bottom:8px">
              在「系统设置 → 通知配置」中配置推送渠道，支持钉钉、飞书、企业微信、Telegram、Bark、邮件等。
              配置后可在「预警中心」点击"测试"按钮验证。
            </p>
          </el-collapse-item>

          <el-collapse-item title="定时调度（可选）" name="scheduler">
            <p style="color:var(--el-text-color-secondary)">
              在「系统设置 → 调度配置」中启用定时任务，系统会在指定时间自动对自选股执行分析。
              设置调度时间（如 18:00 收盘后），填入关注股票代码，点击"启动"即可。
            </p>
          </el-collapse-item>
        </el-collapse>
      </el-card>
    </div>

    <div v-show="activeStep === 2">
      <el-card shadow="hover" style="margin-bottom:20px">
        <template #header>
          <div style="display:flex;align-items:center;gap:8px">
            <el-icon color="var(--el-color-primary)" :size="20"><Pointer /></el-icon>
            <span style="font-weight:600">第三步：开始使用</span>
          </div>
        </template>

        <el-timeline>
          <el-timeline-item timestamp="工作台" placement="top" size="large">
            <el-card shadow="hover">
              <h4 style="margin:0 0 8px">首页概览 & 一键分析</h4>
              <el-steps direction="vertical" :active="4" :space="50" finish-status="success">
                <el-step title="查看大盘指数" description="页面顶部自动展示上证、深证、创业板实时行情" />
                <el-step title="搜索股票" description="在搜索框输入股票代码或名称（如：贵州茅台、600519）" />
                <el-step title="点击「开始分析」" description="系统调用 LLM 生成完整分析报告" />
                <el-step title="查看报告" description="情绪评分仪表盘 + 买入/卖出/持有建议 + 目标价 + 风险提示 + Markdown 完整报告" />
              </el-steps>
              <el-alert type="success" :closable="false" style="margin-top:12px">
                首次使用建议：先在工作台搜索一只熟悉的股票进行分析，感受 AI 报告的完整度
              </el-alert>
            </el-card>
          </el-timeline-item>

          <el-timeline-item timestamp="Agent 问股" placement="top" size="large">
            <el-card shadow="hover">
              <h4 style="margin:0 0 8px">与 AI Agent 对话</h4>
              <p style="color:var(--el-text-color-secondary)">
                Agent 支持多轮对话，可调用行情查询、技术分析、回测等工具。支持 SSE 流式响应，实时显示思考过程。
              </p>
              <el-divider content-position="left">使用方式</el-divider>
              <el-steps direction="vertical" :active="3" :space="50" finish-status="success">
                <el-step title="选择策略（可选）" description="左侧下拉框选择特定技能，如技术分析、决策建议等" />
                <el-step title="输入问题" description="如「分析贵州茅台近期走势」「当前市场风险如何」" />
                <el-step title="查看流式响应" description="Agent 实时输出分析结果，支持 Markdown 渲染" />
              </el-steps>
            </el-card>
          </el-timeline-item>

          <el-timeline-item timestamp="选股筛选" placement="top" size="large">
            <el-card shadow="hover">
              <h4 style="margin:0 0 8px">策略筛选股票</h4>
              <el-steps direction="vertical" :active="3" :space="50" finish-status="success">
                <el-step title="查看筛选引擎状态" description="页面顶部显示是否启用" />
                <el-step title="选择策略" description="切换 Tab 选择不同的筛选策略" />
                <el-step title="点击「执行筛选」" description="查看筛选结果，含评分和理由" />
              </el-steps>
              <p style="color:var(--el-text-color-secondary);margin-top:8px">
                下方还有市场热点卡片，点击可查看热点详情和相关股票。
              </p>
            </el-card>
          </el-timeline-item>

          <el-timeline-item timestamp="投资组合" placement="top" size="large">
            <el-card shadow="hover">
              <h4 style="margin:0 0 8px">管理持仓和交易</h4>
              <el-steps direction="vertical" :active="2" :space="50" finish-status="success">
                <el-step title="买入操作" description="点击「买入」按钮，搜索股票 → 填写价格、数量 → 确认" />
                <el-step title="卖出操作" description="点击「卖出」按钮，同样填写交易信息" />
              </el-steps>
              <el-descriptions :column="2" border size="small" style="margin-top:12px">
                <el-descriptions-item label="顶部统计">总市值、总盈亏、持仓数量、总收益率</el-descriptions-item>
                <el-descriptions-item label="持仓明细">代码、名称、数量、成本价、现价、浮动盈亏</el-descriptions-item>
                <el-descriptions-item label="交易记录">历史买入/卖出明细</el-descriptions-item>
                <el-descriptions-item label="成本追踪">支持 FIFO 分批成本追踪</el-descriptions-item>
              </el-descriptions>
            </el-card>
          </el-timeline-item>

          <el-timeline-item timestamp="决策信号" placement="top" size="large">
            <el-card shadow="hover">
              <h4 style="margin:0 0 8px">查看买卖信号</h4>
              <p style="color:var(--el-text-color-secondary);margin-bottom:8px">
                系统从分析报告中自动提取买卖信号，你可以追踪信号后续表现。
              </p>
              <el-descriptions :column="1" border size="small">
                <el-descriptions-item label="信号列表">卡片式展示，含评分、置信度、操作类型</el-descriptions-item>
                <el-descriptions-item label="筛选">支持按股票、操作类型（买入/卖出/持有）、状态筛选</el-descriptions-item>
                <el-descriptions-item label="信号详情">点击卡片打开侧边栏，查看理由、入场价、止损价、目标价</el-descriptions-item>
                <el-descriptions-item label="操作">可「采纳」或「拒绝」待处理信号</el-descriptions-item>
                <el-descriptions-item label="反馈">提交对该信号的反馈和评分</el-descriptions-item>
                <el-descriptions-item label="结果追踪">查看信号发出后的实际收益、是否命中目标价</el-descriptions-item>
              </el-descriptions>
            </el-card>
          </el-timeline-item>

          <el-timeline-item timestamp="回测分析" placement="top" size="large">
            <el-card shadow="hover">
              <h4 style="margin:0 0 8px">验证历史信号</h4>
              <el-steps direction="vertical" :active="2" :space="50" finish-status="success">
                <el-step title="执行回测" description="输入分析 ID，点击「执行回测」" />
                <el-step title="查看结果" description="胜率、总收益、最大回撤、交易明细" />
              </el-steps>
              <p style="color:var(--el-text-color-secondary);margin-top:8px">
                回测评估分析信号在历史数据中的实际表现，帮助你判断信号的可靠性。
              </p>
            </el-card>
          </el-timeline-item>

          <el-timeline-item timestamp="预警中心" placement="top" size="large">
            <el-card shadow="hover">
              <h4 style="margin:0 0 8px">设置价格/指标告警</h4>
              <el-steps direction="vertical" :active="3" :space="50" finish-status="success">
                <el-step title="新建规则" description="点击「新建规则」按钮" />
                <el-step title="填写条件" description="选择类型（价格突破/涨跌幅/成交量等），填写条件 JSON" />
                <el-step title="启用规则" description="创建后在列表中开启开关，触发时推送通知" />
              </el-steps>
              <p style="color:var(--el-text-color-secondary);margin-top:8px">
                条件示例：价格突破 <code class="dsa-code">{"field":"price","op":">","value":1800}</code>，
                涨跌幅 <code class="dsa-code">{"field":"change_pct","op":">","value":5}</code>
              </p>
            </el-card>
          </el-timeline-item>

          <el-timeline-item timestamp="用量统计" placement="top" size="large">
            <el-card shadow="hover">
              <h4 style="margin:0 0 8px">监控 LLM 调用成本</h4>
              <p style="color:var(--el-text-color-secondary)">
                用量统计页展示 Token 消耗、费用估算、模型分布和调用记录。
                可切换日/周/月维度查看，帮助你控制 API 成本。
              </p>
            </el-card>
          </el-timeline-item>

          <el-timeline-item timestamp="系统设置" placement="top" size="large">
            <el-card shadow="hover">
              <h4 style="margin:0 0 8px">全局配置管理</h4>
              <el-descriptions :column="1" border size="small">
                <el-descriptions-item label="LLM 配置">供应商、模型、API Key、温度、最大 Token</el-descriptions-item>
                <el-descriptions-item label="通知配置">钉钉/飞书/企微/Telegram/Bark/邮件 Webhook</el-descriptions-item>
                <el-descriptions-item label="调度配置">启用/停止定时任务，设置执行时间和自选股</el-descriptions-item>
                <el-descriptions-item label="认证配置">修改登录密码</el-descriptions-item>
                <el-descriptions-item label="情报源">管理 RSS/API 新闻源，测试/抓取/编辑</el-descriptions-item>
                <el-descriptions-item label="配置管理">重载/导出/导入系统配置</el-descriptions-item>
              </el-descriptions>
            </el-card>
          </el-timeline-item>
        </el-timeline>
      </el-card>
    </div>

    <div v-show="activeStep === 3">
      <el-card shadow="hover" style="margin-bottom:20px">
        <template #header>
          <div style="display:flex;align-items:center;gap:8px">
            <el-icon color="var(--el-color-primary)" :size="20"><TrendCharts /></el-icon>
            <span style="font-weight:600">第四步：进阶功能</span>
          </div>
        </template>

        <el-collapse v-model="advancedCollapse">
          <el-collapse-item title="定时自动分析" name="auto">
            <p style="color:var(--el-text-color-secondary);margin-bottom:8px">
              启用调度后，系统每天在指定时间自动对自选股执行分析，无需手动操作。
            </p>
            <el-steps direction="vertical" :active="3" :space="50" finish-status="success">
              <el-step title="前往「系统设置 → 调度配置」" />
              <el-step title="开启调度开关，设置时间（建议 18:00 收盘后）" />
              <el-step title="填入自选股代码，每行一个，如 600519" />
            </el-steps>
          </el-collapse-item>

          <el-collapse-item title="多 Agent 协作" name="agent">
            <p style="color:var(--el-text-color-secondary);margin-bottom:8px">
              Agent 支持多智能体协作模式，各专业 Agent 分工处理不同类型的问题：
            </p>
            <el-descriptions :column="2" border size="small">
              <el-descriptions-item label="技术分析 Agent">K线形态、指标计算</el-descriptions-item>
              <el-descriptions-item label="决策建议 Agent">买入/卖出建议</el-descriptions-item>
              <el-descriptions-item label="风险评估 Agent">风险提示与风控</el-descriptions-item>
              <el-descriptions-item label="策略生成 Agent">交易策略推荐</el-descriptions-item>
              <el-descriptions-item label="组合管理 Agent">持仓优化建议</el-descriptions-item>
              <el-descriptions-item label="情报分析 Agent">新闻与情报解读</el-descriptions-item>
            </el-descriptions>
            <p style="color:var(--el-text-color-secondary);margin-top:8px">
              在 <code class="dsa-code">config.toml</code> 的 <code class="dsa-code">[agent]</code> 段设置
              <code class="dsa-code">arch = "multi"</code> 和 <code class="dsa-code">enabled = true</code> 启用。
            </p>
          </el-collapse-item>

          <el-collapse-item title="通知推送" name="push">
            <p style="color:var(--el-text-color-secondary);margin-bottom:8px">
              支持 14+ 种通知渠道，告警触发时自动推送：
            </p>
            <el-table :data="notifChannels" stripe size="small" style="margin-bottom:12px">
              <el-table-column prop="name" label="渠道" width="120" />
              <el-table-column prop="desc" label="说明" />
              <el-table-column prop="config" label="配置项" width="200" />
            </el-table>
            <el-divider content-position="left">通知分级路由</el-divider>
            <el-descriptions :column="1" border size="small">
              <el-descriptions-item label="critical（严重）">推送至全部已配置渠道</el-descriptions-item>
              <el-descriptions-item label="warning（警告）">推送至即时通讯 + 邮件</el-descriptions-item>
              <el-descriptions-item label="info（信息）">仅记录日志</el-descriptions-item>
            </el-descriptions>
          </el-collapse-item>

          <el-collapse-item title="情报源管理" name="intel">
            <p style="color:var(--el-text-color-secondary);margin-bottom:8px">
              在「系统设置 → 情报源配置」中添加 RSS/API 新闻源，系统自动抓取和去重。
            </p>
            <el-steps direction="vertical" :active="4" :space="50" finish-status="success">
              <el-step title="点击「查看模板」了解支持的源类型" />
              <el-step title="点击「应用默认」快速添加预设源" />
              <el-step title="或手动「新建源」，填写名称/类型/URL" />
              <el-step title="点击「抓取已启用」开始获取新闻" />
            </el-steps>
          </el-collapse-item>

          <el-collapse-item title="敏感信息保护" name="security">
            <p style="color:var(--el-text-color-secondary);margin-bottom:8px">
              所有密钥/密码都支持环境变量引用，避免明文写入配置文件：
            </p>
            <el-table :data="envFields" stripe size="small">
              <el-table-column prop="field" label="配置字段" width="160" />
              <el-table-column prop="envField" label="环境变量字段" width="180" />
              <el-table-column prop="example" label="环境变量名" width="200" />
            </el-table>
            <el-alert type="success" :closable="false" style="margin-top:12px">
              环境变量优先级高于配置文件中的明文值，推荐生产环境使用
            </el-alert>
          </el-collapse-item>
        </el-collapse>
      </el-card>

      <el-card shadow="hover">
        <template #header>
          <div style="display:flex;align-items:center;gap:8px">
            <el-icon color="var(--el-color-warning)" :size="20"><WarningFilled /></el-icon>
            <span style="font-weight:600">常见问题</span>
          </div>
        </template>

        <el-collapse>
          <el-collapse-item title="AI 分析报错？" name="faq1">
            <ul style="margin:0;padding-left:20px;color:var(--el-text-color-regular);line-height:2">
              <li>确认 API Key 已设置：在「系统设置 → LLM配置」点击「测试连接」</li>
              <li>确认网络能访问 LLM API（DeepSeek: api.deepseek.com）</li>
              <li>如需代理，编辑 <code class="dsa-code">config.toml</code> 中的 <code class="dsa-code">[proxy]</code> 段</li>
            </ul>
          </el-collapse-item>
          <el-collapse-item title="行情数据为空？" name="faq2">
            <ul style="margin:0;padding-left:20px;color:var(--el-text-color-regular);line-height:2">
              <li>A 股数据来自东方财富/腾讯/新浪公开接口，需联网</li>
              <li>非交易时段（盘前/盘后/节假日）部分数据可能为空</li>
              <li>确认 <code class="dsa-code">config.toml</code> 中 <code class="dsa-code">[stock] enable_realtime = true</code></li>
            </ul>
          </el-collapse-item>
          <el-collapse-item title="数据库连接失败？" name="faq3">
            <ul style="margin:0;padding-left:20px;color:var(--el-text-color-regular);line-height:2">
              <li>确认 MySQL/MariaDB 正在运行</li>
              <li>确认 <code class="dsa-code">config.toml</code> 中数据库密码正确</li>
              <li>确认数据库 <code class="dsa-code">dsa</code> 已创建</li>
            </ul>
          </el-collapse-item>
          <el-collapse-item title="前端页面空白？" name="faq4">
            <ul style="margin:0;padding-left:20px;color:var(--el-text-color-regular);line-height:2">
              <li>确认后端在 8000 端口运行：访问 <el-link type="primary" href="/health" target="_blank">/health</el-link></li>
              <li>打开浏览器开发者工具 Network 面板，检查请求是否报错</li>
            </ul>
          </el-collapse-item>
        </el-collapse>
      </el-card>
    </div>

    <div style="display:flex;justify-content:center;margin-top:24px;gap:12px">
      <el-button v-if="activeStep > 0" @click="activeStep--">
        <el-icon><ArrowLeft /></el-icon> 上一步
      </el-button>
      <el-button v-if="activeStep < 3" type="primary" @click="activeStep++">
        下一步 <el-icon><ArrowRight /></el-icon>
      </el-button>
      <el-button v-if="activeStep === 3" type="success" @click="$router.push('/')">
        开始使用 <el-icon><Right /></el-icon>
      </el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const activeStep = ref(0)
const configCollapse = ref(['llm'])
const advancedCollapse = ref(['auto'])

const notifChannels = ref([
  { name: '钉钉', desc: 'Webhook 群机器人', config: 'dingtalk_webhook' },
  { name: '飞书', desc: 'Webhook 群机器人', config: 'feishu_webhook' },
  { name: '企业微信', desc: 'Webhook 群机器人', config: 'wecom_webhook' },
  { name: 'Telegram', desc: 'Bot API 推送', config: 'telegram_bot_token + chat_id' },
  { name: 'Bark', desc: 'iOS 推送', config: 'bark_url' },
  { name: '邮件', desc: 'SMTP 发送', config: 'email_smtp_*' },
  { name: 'Discord', desc: 'Webhook', config: 'discord_webhook' },
  { name: 'Slack', desc: 'Webhook', config: 'slack_webhook' },
  { name: 'PushPlus', desc: '微信推送', config: 'pushplus_token' },
  { name: 'ServerChan', desc: 'Server酱', config: 'serverchan_token' },
])

const envFields = ref([
  { field: 'database.password', envField: 'database.password_env', example: 'DSA_DB_PASSWORD' },
  { field: 'llm.api_key', envField: 'llm.api_key_env', example: 'DEEPSEEK_API_KEY' },
  { field: 'email_pass', envField: 'email_pass_env', example: 'DSA_EMAIL_PASS' },
  { field: 'serper_api_key', envField: 'serper_api_key_env', example: 'SERPER_API_KEY' },
])
</script>

<style scoped lang="scss">
.guide-view {
  max-width: 960px;
  margin: 0 auto;
}
.el-code {
  display: inline;
  padding: 2px 6px;
  border-radius: 4px;
  font-family: 'Menlo', 'Monaco', 'Courier New', monospace;
  font-size: 13px;
  background: var(--el-fill-color-light);
  color: var(--el-color-danger);
}
</style>
