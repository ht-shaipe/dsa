import { defineStore } from 'pinia'

export interface ChatMessage {
  role: 'user' | 'assistant'
  content: string
  timestamp: number
}

export const useChatStore = defineStore('chat', {
  state: () => ({
    messages: [] as ChatMessage[],
    isStreaming: false,
    currentSkill: '',
    skills: [] as any[],
  }),
  actions: {
    addUserMessage(content: string) {
      this.messages.push({ role: 'user', content, timestamp: Date.now() })
    },
    addAssistantMessage(content: string) {
      this.messages.push({ role: 'assistant', content, timestamp: Date.now() })
    },
    updateLastAssistant(content: string) {
      const last = this.messages[this.messages.length - 1]
      if (last && last.role === 'assistant') {
        last.content = content
      }
    },
    clearMessages() {
      this.messages = []
    },
    setStreaming(v: boolean) {
      this.isStreaming = v
    },
    setSkills(skills: any[]) {
      this.skills = skills
    },
    setCurrentSkill(skill: string) {
      this.currentSkill = skill
    },
  },
})
