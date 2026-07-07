import { defineStore } from 'pinia'

export interface ChatMessage {
  role: 'user' | 'assistant'
  content: string
  timestamp: number
}

export interface ChatSession {
  session_id: string
  title: string
  messages: ChatMessage[]
  skill: string
  loaded: boolean
}

function genId() {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 8)
}

export const useChatStore = defineStore('chat', {
  state: () => ({
    sessions: [] as ChatSession[],
    currentSessionIdx: 0,
    isStreaming: false,
    currentSkill: '',
    skills: [] as any[],
  }),
  actions: {
    addSession(sessionId?: string) {
      const sid = sessionId || genId()
      this.sessions.push({ session_id: sid, title: '', messages: [], skill: this.currentSkill, loaded: true })
      this.currentSessionIdx = this.sessions.length - 1
    },
    switchSession(idx: number) {
      if (idx >= 0 && idx < this.sessions.length) {
        this.currentSessionIdx = idx
      }
    },
    deleteSession(idx: number) {
      this.sessions.splice(idx, 1)
      if (this.sessions.length === 0) {
        this.addSession()
      } else if (this.currentSessionIdx >= this.sessions.length) {
        this.currentSessionIdx = this.sessions.length - 1
      } else if (this.currentSessionIdx > idx) {
        this.currentSessionIdx--
      } else if (this.currentSessionIdx === idx) {
        this.currentSessionIdx = Math.min(idx, this.sessions.length - 1)
      }
    },
    updateSessionTitle(idx: number, title: string) {
      if (idx >= 0 && idx < this.sessions.length) {
        this.sessions[idx].title = title
      }
    },
    setSessionMessages(idx: number, messages: ChatMessage[]) {
      if (idx >= 0 && idx < this.sessions.length) {
        this.sessions[idx].messages = messages
        this.sessions[idx].loaded = true
        if (!this.sessions[idx].title && messages.length > 0) {
          const first = messages.find(m => m.role === 'user')
          if (first) this.sessions[idx].title = first.content.slice(0, 20)
        }
      }
    },
    addUserMessage(content: string) {
      if (this.sessions.length === 0) this.addSession()
      this.sessions[this.currentSessionIdx].messages.push({ role: 'user', content, timestamp: Date.now() })
    },
    addAssistantMessage(content: string) {
      if (this.sessions.length === 0) this.addSession()
      this.sessions[this.currentSessionIdx].messages.push({ role: 'assistant', content, timestamp: Date.now() })
    },
    updateLastAssistant(content: string) {
      const msgs = this.sessions[this.currentSessionIdx]?.messages
      if (!msgs) return
      const last = msgs[msgs.length - 1]
      if (last && last.role === 'assistant') {
        last.content = content
      }
    },
    clearMessages() {
      if (this.sessions[this.currentSessionIdx]) {
        this.sessions[this.currentSessionIdx].messages = []
        this.sessions[this.currentSessionIdx].title = ''
      }
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
