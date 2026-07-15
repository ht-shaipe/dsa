//! Agent 记忆模块 - 对齐原项目 agent/memory.py

use tube::Value;

/// Agent 记忆管理
/// 管理对话消息的存储、检索和截断
pub struct AgentMemory {
    messages: Vec<Value>,
}

impl AgentMemory {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    /// 添加消息
    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(value!({
            "role": role,
            "content": content,
        }));
    }

    /// 获取所有消息
    pub fn get_messages(&self) -> &[Value] {
        &self.messages
    }

    /// 清空消息
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// 获取最近N条消息
    pub fn recent(&self, n: usize) -> Vec<Value> {
        let len = self.messages.len();
        if n >= len {
            self.messages.clone()
        } else {
            self.messages[len - n..].to_vec()
        }
    }

    /// 截断消息以保持在token预算内
    /// 近似估算: 1个中文字符 ≈ 1个token
    pub fn truncate(&mut self, max_tokens: usize) {
        let mut total_chars: usize = 0;
        // 从后往前计算
        let mut keep_from = self.messages.len();
        for (i, msg) in self.messages.iter().rev().enumerate() {
            let content_chars = msg
                .get("content")
                .and_then(|c| c.as_str())
                .map(|s| s.len())
                .unwrap_or(0);
            total_chars += content_chars;
            if total_chars > max_tokens {
                keep_from = self.messages.len() - i;
                break;
            }
        }
        if keep_from > 0 {
            self.messages = self.messages[keep_from..].to_vec();
        }
    }
}

impl Default for AgentMemory {
    fn default() -> Self {
        Self::new()
    }
}
