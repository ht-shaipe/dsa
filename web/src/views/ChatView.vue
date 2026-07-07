<template>
  <div class="chat-page">
    <el-row :gutter="20">
      <el-col :span="6">
        <el-card shadow="hover">
          <template #header>策略选择</template>
          <el-select v-model="chatStore.currentSkill" placeholder="选择策略" style="width:100%" @change="loadSkills">
            <el-option label="通用对话" value="" />
            <el-option v-for="s in chatStore.skills" :key="s.name || s.id" :label="s.label || s.name" :value="s.name || s.id" />
          </el-select>
        </el-card>
      </el-col>
      <el-col :span="18">
        <el-card shadow="hover" class="chat-main">
          <div class="chat-messages" ref="messagesRef">
            <div v-for="(msg, idx) in chatStore.messages" :key="idx" :class="['chat-bubble', msg.role]">
              <div class="bubble-content">
                <MarkdownRenderer v-if="msg.role === 'assistant'" :content="msg.content" />
                <template v-else>{{ msg.content }}</template>
              </div>
            </div>
            <div v-if="chatStore.isStreaming" class="chat-bubble assistant">
              <div class="bubble-content">
                <el-icon class="is-loading"><Loading /></el-icon> 思考中...
              </div>
            </div>
          </div>
          <div class="chat-input-area">
            <el-input
              v-model="inputMsg"
              placeholder="输入消息..."
              @keyup.enter="sendMessage"
              :disabled="chatStore.isStreaming"
            />
            <el-button type="primary" :loading="chatStore.isStreaming" @click="sendMessage" :disabled="!inputMsg.trim()">
              发送
            </el-button>
          </div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, nextTick } from 'vue'
import MarkdownRenderer from '@/components/common/MarkdownRenderer.vue'
import { useChatStore } from '@/stores/chat'
import { useAuthStore } from '@/stores/auth'
import { agentApi } from '@/api/agent'
import { ElMessage } from 'element-plus'

const chatStore = useChatStore()
const authStore = useAuthStore()
const inputMsg = ref('')
const messagesRef = ref<HTMLElement>()

async function loadSkills() {
  try {
    const res: any = await agentApi.skills()
    chatStore.setSkills(res.data || [])
  } catch { /* ignore */ }
}

async function sendMessage() {
  const msg = inputMsg.value.trim()
  if (!msg || chatStore.isStreaming) return

  chatStore.addUserMessage(msg)
  chatStore.setStreaming(true)
  inputMsg.value = ''
  await scrollBottom()

  try {
    const token = authStore.token
    const response = await fetch(`/api/v1/agent/chat/stream?token=${token}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ message: msg, skill: chatStore.currentSkill }),
    })

    if (!response.ok || !response.body) {
      throw new Error('Stream request failed')
    }

    let fullContent = ''
    chatStore.addAssistantMessage('')
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
            fullContent += event.content
            chatStore.updateLastAssistant(fullContent)
            await scrollBottom()
          } else if (event.type === 'error') {
            ElMessage.error(event.content || 'Stream error')
          }
        } catch { /* skip unparseable lines */ }
      }
    }

    if (!fullContent) {
      chatStore.updateLastAssistant('(无响应内容)')
    }
  } catch (e: any) {
    try {
      const res: any = await agentApi.chat(msg)
      const content = res.data?.content || res.data?.message || res.data || JSON.stringify(res.data)
      chatStore.addAssistantMessage(typeof content === 'string' ? content : JSON.stringify(content))
    } catch {
      chatStore.addAssistantMessage('请求失败，请稍后重试')
    }
  } finally {
    chatStore.setStreaming(false)
    await scrollBottom()
  }
}

async function scrollBottom() {
  await nextTick()
  if (messagesRef.value) {
    messagesRef.value.scrollTop = messagesRef.value.scrollHeight
  }
}

onMounted(() => {
  loadSkills()
})
</script>

<style scoped lang="scss">
.chat-main {
  display: flex;
  flex-direction: column;
  height: calc(100vh - 140px);
}
.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  min-height: 400px;
}
.chat-bubble {
  margin-bottom: 16px;
  display: flex;
  &.user { justify-content: flex-end; }
  &.assistant { justify-content: flex-start; }
}
.bubble-content {
  max-width: 70%;
  padding: 10px 16px;
  border-radius: 12px;
  font-size: 14px;
  line-height: 1.6;
  .user & {
    background: var(--el-color-primary);
    color: #fff;
    border-bottom-right-radius: 4px;
  }
  .assistant & {
    background: var(--el-fill-color-light);
    color: var(--el-text-color-primary);
    border-bottom-left-radius: 4px;
  }
}
.chat-input-area {
  display: flex;
  gap: 12px;
  padding: 16px 0 0;
  border-top: 1px solid var(--el-border-color-lighter);
}
</style>
