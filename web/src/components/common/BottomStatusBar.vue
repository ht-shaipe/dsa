<template>
  <div class="bottom-status-bar">
    <div class="status-left">
      <template v-if="taskStore.hasRunningTasks">
        <el-divider direction="vertical" />
        <el-popover
          placement="top-end"
          :width="340"
          trigger="click"
          :visible="popoverVisible"
          @update:visible="popoverVisible = $event"
        >
          <template #reference>
            <div class="task-inline">
              <el-icon v-if="anyPaused" :size="13" color="var(--el-color-warning)"><VideoPause /></el-icon>
              <el-icon v-else class="is-loading" :size="13" color="var(--el-color-primary)"><Loading /></el-icon>
              <span class="task-name" v-for="task in taskStore.runningTasks" :key="task.task">
                {{ getTaskLabel(task.task) }}
                <template v-if="task.total > 0">
                  {{ task.done.toLocaleString() }}/{{ task.total.toLocaleString() }}
                  ({{ (task.done / task.total * 100).toFixed(1) }}%)
                </template>
                <template v-else-if="task.phase && task.phase !== 'done'">
                  {{ getPhaseLabel(task.phase) }}
                </template>
                <template v-else-if="task.done > 0">{{ task.done.toLocaleString() }}</template>
                <template v-if="task.current_code"> · {{ task.current_name || task.current_code }}</template>
              </span>
            </div>
          </template>
          <div class="popover-content">
            <div v-for="task in taskStore.runningTasks" :key="task.task" class="popover-task">
              <div class="popover-task-header">
                <span class="popover-task-name">{{ getTaskLabel(task.task) }}</span>
                <el-tag v-if="task.paused" type="warning" size="small">已暂停</el-tag>
              </div>
              <el-progress
                v-if="task.total > 0"
                :percentage="Math.round(task.done / task.total * 100)"
                :stroke-width="8"
                :status="task.paused ? 'warning' : undefined"
              />
              <el-progress
                v-else
                :percentage="100"
                :stroke-width="8"
                :status="task.paused ? 'warning' : undefined"
                :indeterminate="true"
              />
              <div class="popover-task-details">
                <span v-if="task.phase && task.phase !== 'done'">{{ getPhaseLabel(task.phase) }}</span>
                <span v-if="task.total > 0">{{ task.done.toLocaleString() }}/{{ task.total.toLocaleString() }}</span>
                <span v-if="task.failed > 0" class="failed-text">失败: {{ task.failed }}</span>
              </div>
              <div class="popover-task-actions">
                <el-button v-if="task.paused" type="primary" size="small" @click="resumeTask(task.task)">
                  <el-icon><VideoPlay /></el-icon> 继续
                </el-button>
                <el-button v-else type="warning" size="small" @click="pauseTask(task.task)">
                  <el-icon><VideoPause /></el-icon> 暂停
                </el-button>
                <el-popconfirm title="确定要停止该任务吗？停止后将无法继续。" @confirm="stopTask(task.task)">
                  <template #reference>
                    <el-button type="danger" size="small"><el-icon><Close /></el-icon> 停止</el-button>
                  </template>
                </el-popconfirm>
              </div>
            </div>
          </div>
        </el-popover>
      </template>
    </div>
    <div class="status-right">
      <span class="status-item mode-tag">{{ isTauri ? 'Desktop' : 'Web' }}</span>
      <el-divider direction="vertical" />
      <span class="status-item version-tag">v{{ appVersion }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { Loading, VideoPause, VideoPlay, Close } from '@element-plus/icons-vue'
import { useTaskStore, getTaskLabel, getPhaseLabel } from '@/stores/task'

declare const __APP_VERSION__: string

const taskStore = useTaskStore()
const popoverVisible = ref(false)

const isTauri = typeof window !== 'undefined' &&
  !!((window as any).__TAURI_INTERNALS__ || (window as any).__TAURI__)

const appVersion = typeof __APP_VERSION__ !== 'undefined' ? __APP_VERSION__ : '0.1.0'

const anyPaused = computed(() => taskStore.runningTasks.some(t => t.paused))

async function pauseTask(task: string) { await taskStore.pauseTask(task) }
async function resumeTask(task: string) { await taskStore.resumeTask(task) }
async function stopTask(task: string) { popoverVisible.value = false; await taskStore.stopTask(task) }
</script>

<style scoped lang="scss">
.bottom-status-bar {
  height: 32px;
  background: var(--el-bg-color);
  border-top: 1px solid var(--el-border-color-lighter);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 14px;
  font-size: 13px;
  color: var(--el-text-color-secondary);
  flex-shrink: 0;
  user-select: none;
  position: relative;
  z-index: 10;
  box-shadow: 0 -2px 8px -2px rgba(0, 0, 0, 0.06), 0 -1px 2px -1px rgba(0, 0, 0, 0.04);
}

.status-left,
.status-right {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  overflow: hidden;
}

.status-item {
  display: flex;
  align-items: center;
  gap: 5px;
  white-space: nowrap;
}

.task-inline {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  min-width: 0;
  overflow: hidden;
  &:hover { opacity: 0.85; }
}

.task-name {
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.popover-content {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.popover-task-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 6px;
}

.popover-task-name {
  font-size: 13px;
  font-weight: 600;
}

.popover-task-details {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 4px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.failed-text { color: var(--el-color-danger); }

.popover-task-actions {
  display: flex;
  gap: 8px;
  margin-top: 8px;
}

.mode-tag, .version-tag { opacity: 0.7; }

:deep(.el-divider--vertical) {
  height: 14px;
  margin: 0 4px;
  border-color: var(--el-border-color-lighter);
}
</style>
