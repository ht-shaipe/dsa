<template>
  <el-dialog
    v-model="visible"
    title="初始化股票池"
    width="520px"
    :close-on-click-modal="false"
    @closed="onClosed"
  >
    <p style="margin:0 0 16px;color:var(--el-text-color-secondary)">
      从东方财富获取全量A股列表，按以下规则过滤后写入股票池。已有股票将被更新。
    </p>

    <el-form :model="form" label-width="130px">
      <el-divider content-position="left">市场板块</el-divider>
      <el-form-item label="选择板块">
        <el-checkbox-group v-model="form.boards">
          <el-checkbox label="沪市主板" value="sh_main" />
          <el-checkbox label="深市主板" value="sz_main" />
          <el-checkbox label="创业板" value="sz_gem" />
          <el-checkbox label="科创板" value="sh_kj" />
          <el-checkbox label="北交所" value="bj_main" />
        </el-checkbox-group>
      </el-form-item>

      <el-divider content-position="left">风险过滤</el-divider>
      <el-form-item label="排除ST股票">
        <el-switch v-model="form.excludeSt" />
        <span style="margin-left:8px;color:var(--el-text-color-secondary)">名称含ST/*ST</span>
      </el-form-item>
      <el-form-item label="排除退市风险">
        <el-switch v-model="form.excludeDelisting" />
        <span style="margin-left:8px;color:var(--el-text-color-secondary)">名称含退市/退</span>
      </el-form-item>
      <el-form-item label="排除次新股">
        <el-switch v-model="form.excludeNew" />
        <span style="margin-left:8px;color:var(--el-text-color-secondary)">上市不足60天</span>
      </el-form-item>
    </el-form>

    <template #footer>
      <el-button @click="visible = false">取消</el-button>
      <el-button type="primary" :loading="submitting" :disabled="form.boards.length === 0" @click="handleSubmit">
        开始初始化
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue'
import { ElMessage } from 'element-plus'
import { stockPoolApi } from '@/api/stockPool'
import { useTaskStore } from '@/stores/task'

const taskStore = useTaskStore()

const visible = defineModel<boolean>('visible', { default: false })
const emit = defineEmits<{
  (e: 'started'): void
}>()

const submitting = ref(false)

const form = reactive({
  boards: ['sh_main', 'sz_main', 'sz_gem'] as string[],
  excludeSt: true,
  excludeDelisting: true,
  excludeNew: true,
})

function resetForm() {
  form.boards = ['sh_main', 'sz_main', 'sz_gem']
  form.excludeSt = true
  form.excludeDelisting = true
  form.excludeNew = true
}

async function handleSubmit() {
  if (form.boards.length === 0) {
    ElMessage.warning('请至少选择一个板块')
    return
  }
  submitting.value = true
  try {
    await stockPoolApi.initPool({
      boards: form.boards,
      exclude_st: form.excludeSt,
      exclude_delisting: form.excludeDelisting,
      exclude_new: form.excludeNew,
    })
    ElMessage.success('股票池初始化已启动，可在底部状态栏查看进度')
    visible.value = false
    emit('started')
    taskStore.refreshAllStatus()
  } catch {
    // error shown by api interceptor
  } finally {
    submitting.value = false
  }
}

function onClosed() {
  resetForm()
}
</script>
