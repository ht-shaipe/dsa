<template>
  <div class="chat-page">
    <div class="chat-sidebar">
      <div class="sidebar-top">
        <el-button type="primary" style="width:100%" @click="newChat">
          <el-icon><Plus /></el-icon> 新对话
        </el-button>
      </div>
      <div class="sidebar-list">
        <el-scrollbar>
          <div
            v-for="(s, idx) in chatStore.sessions"
            :key="idx"
            :class="['session-item', { active: idx === chatStore.currentSessionIdx }]"
            @click="switchSession(idx)"
          >
            <el-icon size="14"><ChatDotRound /></el-icon>
            <span class="session-title">{{ s.title || '新对话' }}</span>
            <el-icon class="session-del" size="12" @click.stop="deleteSession(idx)"><Close /></el-icon>
          </div>
          <div v-if="!chatStore.sessions.length" class="sidebar-empty">暂无对话</div>
        </el-scrollbar>
      </div>
      <div class="sidebar-bottom">
        <el-select v-model="chatStore.currentSkill" placeholder="选择策略" style="width:100%" size="small">
          <el-option label="通用对话" value="" />
          <el-option v-for="s in chatStore.skills" :key="s.name || s.id" :label="s.label || s.name" :value="s.name || s.id" />
        </el-select>
      </div>
    </div>

    <div class="chat-main">
      <div v-if="currentMessages.length === 0" class="chat-empty">
        <div class="empty-icon">
          <component :is="ChatDotRound" :size="48" style="color: var(--el-color-primary-light-3)" />
        </div>
        <h3>AI 问股助手</h3>
        <p>输入股票代码或问题，开始智能分析</p>
        <div class="quick-prompts">
          <el-button v-for="q in quickPrompts" :key="q" size="small" round @click="inputMsg = q; sendMessage()">{{ q }}</el-button>
        </div>
      </div>

      <div v-else class="chat-messages">
        <el-scrollbar ref="messagesScrollbarRef">
          <div class="chat-messages-inner">
            <div v-for="(msg, idx) in currentMessages" :key="idx" :class="['chat-bubble', msg.role]">
              <div class="bubble-avatar">
                <component v-if="msg.role === 'assistant'" :is="Monitor" :size="18" />
                <component v-else :is="User" :size="18" />
              </div>
              <div class="bubble-body">
                <div class="bubble-meta">
                  <span class="bubble-role">{{ msg.role === 'user' ? '我' : 'AI' }}</span>
                  <span class="bubble-time">{{ formatTime(msg.timestamp) }}</span>
                </div>
                <div class="bubble-content">
                  <MarkdownRenderer :content="msg.content" />
                </div>
              </div>
            </div>
            <div v-if="chatStore.isStreaming" class="chat-bubble assistant">
              <div class="bubble-avatar">
                <component :is="Monitor" :size="18" />
              </div>
              <div class="bubble-body">
                <div class="bubble-content streaming">
                  <span class="dot-anim">.</span><span class="dot-anim delay1">.</span><span class="dot-anim delay2">.</span>
                </div>
              </div>
            </div>
          </div>
        </el-scrollbar>
      </div>

      <div class="chat-input-area">
        <el-input
          v-model="inputMsg"
          type="textarea"
          :autosize="{ minRows: 1, maxRows: 4 }"
          placeholder="输入消息，Enter 发送，Shift+Enter 换行..."
          @keydown.enter.exact.prevent="sendMessage"
          :disabled="chatStore.isStreaming"
          resize="none"
        />
        <el-button
          type="primary"
          :icon="Promotion"
          circle
          :loading="chatStore.isStreaming"
          @click="sendMessage"
          :disabled="!inputMsg.trim()"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, nextTick, watch } from 'vue'
import { Promotion, Plus, ChatDotRound, Close, Monitor, User } from '@element-plus/icons-vue'
import type { ScrollbarInstance } from 'element-plus'
import MarkdownRenderer from '@/components/common/MarkdownRenderer.vue'
import { useChatStore, ChatMessage } from '@/stores/chat'
import { useAuthStore } from '@/stores/auth'
import { agentApi } from '@/api/agent'
import { ElMessage } from 'element-plus'

const chatStore = useChatStore()
const authStore = useAuthStore()
const inputMsg = ref('')
const messagesScrollbarRef = ref<ScrollbarInstance>()

const quickPrompts = [
  '分析 SH600519 贵州茅台',
  '大盘今日走势如何？',
  '推荐几只消费龙头股',
  '新能源汽车板块分析',
]

const currentMessages = computed(() => {
  const session = chatStore.sessions[chatStore.currentSessionIdx]
  return session ? session.messages : []
})

function formatTime(ts: number) {
  const d = new Date(ts)
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
}

function newChat() {
  chatStore.addSession()
}

function switchSession(idx: number) {
  chatStore.switchSession(idx)
}

function deleteSession(idx: number) {
  chatStore.deleteSession(idx)
}

async function loadSkills() {
  try {
    const res: any = await agentApi.skills()
    chatStore.setSkills(Array.isArray(res) ? res : [])
  } catch { /* ignore */ }
}

async function loadHistory() {
  try {
    const res: any = await agentApi.history()
    const messages: any[] = Array.isArray(res) ? res : []
    if (!messages.length) return

    const sessionMap = new Map<string, ChatMessage[]>()
    const sessionOrder: string[] = []
    for (const m of messages) {
      const sid = m.session_id || ''
      if (!sessionMap.has(sid)) {
        sessionMap.set(sid, [])
        sessionOrder.push(sid)
      }
      sessionMap.get(sid)!.push({
        role: m.role === 'user' || m.role === 'assistant' ? m.role : 'assistant',
        content: m.content || '',
        timestamp: new Date(m.createTime || Date.now()).getTime(),
      })
    }

    chatStore.sessions = sessionOrder.map(sid => {
      const msgs = sessionMap.get(sid) || []
      const first = msgs.find(m => m.role === 'user')
      return {
        session_id: sid || genId(),
        title: first ? first.content.slice(0, 20) : '对话',
        messages: msgs,
        skill: '',
        loaded: true,
      }
    })
    if (chatStore.sessions.length) {
      chatStore.currentSessionIdx = chatStore.sessions.length - 1
    }
  } catch { /* ignore */ }
}

function genId() {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 8)
}

async function sendMessage() {
  const msg = inputMsg.value.trim()
  if (!msg || chatStore.isStreaming) return

  if (!chatStore.sessions.length) {
    chatStore.addSession()
  }

  const session = chatStore.sessions[chatStore.currentSessionIdx]
  if (!session.title && session.messages.length === 0) {
    chatStore.updateSessionTitle(chatStore.currentSessionIdx, msg.slice(0, 20))
  }

  chatStore.addUserMessage(msg)
  chatStore.setStreaming(true)
  inputMsg.value = ''
  await scrollBottom()

  let streamFailed = false

  try {
    const token = authStore.token
    const apiBase = ((window as any).__TAURI_INTERNALS__ || (window as any).__TAURI__) ? 'http://127.0.0.1:18080' : ''
    const response = await fetch(`${apiBase}/api/v1/agent/chat/stream?token=${token}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ message: msg, skill: chatStore.currentSkill, session_id: chatStore.sessions[chatStore.currentSessionIdx]?.session_id }),
    })

    if (!response.ok || !response.body) {
      throw new Error('Stream request failed')
    }

    let fullContent = ''
    let messageAdded = false
    const reader = response.body.getReader()
    const decoder = new TextDecoder()

    while (true) {
      const { done, value } = await reader.read()
      if (done) break

      const text = decoder.decode(value, { stream: true })
      const lines = text.split('\n')

      for (const line of lines) {
        if (!line.startsWith('data:')) continue
        const jsonStr = line.slice(5).trim()
        if (!jsonStr || jsonStr === '[DONE]') continue

        try {
          const event = JSON.parse(jsonStr)
          if (event.type === 'message' && event.content) {
            if (!messageAdded) {
              chatStore.addAssistantMessage('')
              messageAdded = true
            }
            fullContent += event.content
            chatStore.updateLastAssistant(fullContent)
            await scrollBottom()
          } else if (event.type === 'error') {
            ElMessage.error(event.content || 'Stream error')
          }
        } catch { /* skip */ }
      }
    }

    if (!fullContent && messageAdded) {
      chatStore.updateLastAssistant('(无响应内容)')
    }

    if (!messageAdded) {
      streamFailed = true
    }
  } catch {
    streamFailed = true
  }

  if (streamFailed) {
    try {
      const res: any = await agentApi.chat(msg)
      const content = res?.content || res?.message || (typeof res === 'string' ? res : JSON.stringify(res))
      chatStore.addAssistantMessage(typeof content === 'string' ? content : JSON.stringify(content))
    } catch {
      chatStore.addAssistantMessage('请求失败，请稍后重试')
    }
  }

  chatStore.setStreaming(false)
  await scrollBottom()
}

async function scrollBottom() {
  await nextTick()
  if (messagesScrollbarRef.value) {
    messagesScrollbarRef.value.setScrollTop(999999)
  }
}

watch(currentMessages, () => scrollBottom(), { deep: true })

onMounted(async () => {
  loadSkills()
  await loadHistory()
  if (!chatStore.sessions.length) {
    chatStore.addSession()
  }
})
</script>

<style scoped lang="scss">
.chat-page {
  display: flex;
  height: calc(100vh - 96px);
  background: var(--el-bg-color);
  border-radius: 8px;
  overflow: hidden;
  border: 1px solid var(--el-border-color-lighter);
}

.chat-sidebar {
  width: 220px;
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--el-border-color-lighter);
  background: var(--el-fill-color-lighter);

  .sidebar-top {
    padding: 12px;
    border-bottom: 1px solid var(--el-border-color-lighter);
  }

  .sidebar-list {
    flex: 1;
    overflow: hidden;
    padding: 8px;

    :deep(.el-scrollbar) {
      height: 100%;
    }
  }

  .session-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 10px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
    color: var(--el-text-color-regular);
    transition: background 0.2s;
    margin-bottom: 2px;

    &:hover {
      background: var(--el-fill-color);
    }

    &.active {
      background: var(--el-color-primary-light-9);
      color: var(--el-color-primary);
    }

    .session-title {
      flex: 1;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .session-del {
      opacity: 0;
      transition: opacity 0.2s;
      color: var(--el-text-color-secondary);
      &:hover { color: var(--el-color-danger); }
    }

    &:hover .session-del {
      opacity: 1;
    }
  }

  .sidebar-empty {
    text-align: center;
    color: var(--el-text-color-placeholder);
    font-size: 13px;
    padding: 24px 0;
  }

  .sidebar-bottom {
    padding: 12px;
    border-top: 1px solid var(--el-border-color-lighter);
  }
}

.chat-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.chat-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--el-text-color-secondary);

  .empty-icon {
    margin-bottom: 16px;
    display: flex;
    justify-content: center;

    :deep(svg) {
      width: 48px;
      height: 48px;
    }
  }

  h3 {
    margin: 0 0 8px;
    font-size: 20px;
    color: var(--el-text-color-primary);
  }

  p {
    margin: 0 0 24px;
    font-size: 14px;
  }

  .quick-prompts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    justify-content: center;
    max-width: 480px;
  }
}

.chat-messages {
  flex: 1;
  overflow: hidden;
  padding: 0;

  :deep(.el-scrollbar) {
    height: 100%;
  }
}

.chat-messages-inner {
  padding: 20px 24px;
}

.chat-bubble {
  display: flex;
  gap: 10px;
  margin-bottom: 20px;

  &.user {
    flex-direction: row-reverse;

    .bubble-body {
      align-items: flex-end;
    }

    .bubble-content {
      background: var(--el-color-primary);
      color: #fff;
      border-bottom-right-radius: 4px;
    }

    .bubble-meta {
      flex-direction: row-reverse;
    }
  }

  &.assistant {
    .bubble-content {
      background: var(--el-fill-color-light);
      color: var(--el-text-color-primary);
      border-bottom-left-radius: 4px;

      :deep(.markdown-renderer) {
        h1, h2, h3, h4 {
          color: var(--el-text-color-primary);
        }
        p:last-child {
          margin-bottom: 0;
        }
      }
    }
  }
}

.bubble-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  background: var(--el-fill-color);
  color: var(--el-text-color-secondary);

  :deep(svg) {
    width: 18px;
    height: 18px;
  }

  .user & {
    background: var(--el-color-primary-light-8);
    color: var(--el-color-primary);
  }

  .assistant & {
    background: var(--el-color-success-light-8);
    color: var(--el-color-success);
  }
}

.bubble-body {
  display: flex;
  flex-direction: column;
  max-width: 75%;
  gap: 4px;
}

.bubble-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 4px;
}

.bubble-role {
  font-size: 12px;
  font-weight: 500;
  color: var(--el-text-color-secondary);
}

.bubble-time {
  font-size: 11px;
  color: var(--el-text-color-placeholder);
}

.bubble-content {
  padding: 10px 14px;
  border-radius: 12px;
  font-size: 14px;
  line-height: 1.7;
  word-break: break-word;
}

.streaming {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 12px 16px;
  font-size: 16px;
  letter-spacing: 4px;
}

.dot-anim {
  animation: dotBlink 1.4s infinite;
  &.delay1 { animation-delay: 0.2s; }
  &.delay2 { animation-delay: 0.4s; }
}

@keyframes dotBlink {
  0%, 80%, 100% { opacity: 0.3; }
  40% { opacity: 1; }
}

.chat-input-area {
  display: flex;
  align-items: flex-end;
  gap: 12px;
  padding: 16px 24px;
  border-top: 1px solid var(--el-border-color-lighter);
  background: var(--el-bg-color);

  :deep(.el-textarea__inner) {
    border-radius: 8px;
    padding: 10px 14px;
    font-size: 14px;
  }

  .el-button.is-circle {
    width: 36px;
    height: 36px;
    flex-shrink: 0;
  }
}

html.dark {
  .chat-page {
    border-color: var(--el-border-color);
  }

  .bubble-avatar {
    background: var(--el-fill-color);
  }

  .chat-sidebar {
    background: #1a1a1a;
  }
}
</style>
