<template>
  <el-autocomplete
    v-model="query"
    :fetch-suggestions="searchStock"
    placeholder="输入股票代码或名称"
    clearable
    style="width: 100%"
    @select="handleSelect"
  >
    <template #prefix>
      <el-icon><Search /></el-icon>
    </template>
    <template #default="{ item }">
      <div class="stock-suggest-item">
        <span class="stock-code">{{ item.code }}</span>
        <span class="stock-name">{{ item.name }}</span>
      </div>
    </template>
  </el-autocomplete>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { stockApi } from '@/api/stock'

const emit = defineEmits<{
  select: [code: string, name: string]
}>()

const query = ref('')

async function searchStock(qs: string, cb: (results: any[]) => void) {
  if (!qs || qs.length < 1) {
    cb([])
    return
  }
  try {
    const res: any = await stockApi.search(qs)
    const data = res.data || []
    cb(data.map((item: any) => ({
      value: item.code || item.symbol || '',
      code: item.code || item.symbol || '',
      name: item.name || item.label || '',
    })))
  } catch {
    cb([])
  }
}

function handleSelect(item: any) {
  emit('select', item.code, item.name)
}
</script>

<style scoped lang="scss">
.stock-suggest-item {
  display: flex;
  gap: 12px;
  align-items: center;
}
.stock-code {
  color: var(--el-text-color-secondary);
  font-size: 13px;
}
.stock-name {
  font-size: 14px;
}
</style>
