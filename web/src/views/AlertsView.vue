<template>
  <div class="alerts-view">
    <el-card shadow="hover" style="margin-bottom: 20px">
      <template #header>
        <div style="display:flex;justify-content:space-between;align-items:center">
          <span>预警规则</span>
          <el-button type="primary" @click="openCreateDialog">新建规则</el-button>
        </div>
      </template>
      <el-table :data="rules" stripe style="width:100%">
        <el-table-column prop="id" label="ID" width="60" />
        <el-table-column prop="code" label="代码" width="100" />
        <el-table-column prop="name" label="规则名称" width="150" />
        <el-table-column prop="ruleType" label="类型" width="120">
          <template #default="{ row }">{{ row.rule_type || row.rule_type || '-' }}</template>
        </el-table-column>
        <el-table-column label="条件" min-width="200" show-overflow-tooltip>
          <template #default="{ row }">{{ JSON.stringify(row.condition || {}) }}</template>
        </el-table-column>
        <el-table-column label="启用" width="80">
          <template #default="{ row }">
            <el-switch
              :model-value="!!row.enabled"
              @change="(val: any) => toggleRule(row, !!val)"
              active-text=""
              inactive-text=""
            />
          </template>
        </el-table-column>
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" @click="testRule(row)">测试</el-button>
            <el-button link type="warning" @click="editRule(row)">编辑</el-button>
            <el-popconfirm title="确定删除此规则?" @confirm="deleteRule(row)">
              <template #reference>
                <el-button link type="danger">删除</el-button>
              </template>
            </el-popconfirm>
          </template>
        </el-table-column>
      </el-table>
      <el-empty v-if="!rules.length" description="暂无预警规则" />
    </el-card>

    <el-row :gutter="20">
      <el-col :span="12">
        <el-card shadow="hover">
          <template #header>触发历史</template>
          <el-scrollbar max-height="400px">
            <el-table :data="triggers" stripe style="width:100%">
              <el-table-column prop="id" label="ID" width="60" />
              <el-table-column prop="ruleId" label="规则ID" width="80">
                <template #default="{ row }">{{ row.ruleId || row.rule_id }}</template>
              </el-table-column>
              <el-table-column prop="code" label="代码" width="100" />
              <el-table-column prop="message" label="消息" min-width="200" show-overflow-tooltip />
              <el-table-column prop="triggeredAt" label="触发时间" width="180">
                <template #default="{ row }">{{ row.triggered_at || row.triggered_at || '-' }}</template>
              </el-table-column>
            </el-table>
          </el-scrollbar>
        </el-card>
      </el-col>
      <el-col :span="12">
        <el-card shadow="hover">
          <template #header>
            <div style="display:flex;justify-content:space-between;align-items:center">
              <span>通知渠道</span>
            </div>
          </template>
          <div v-for="ch in channels" :key="ch.key || ch.name" class="channel-item">
            <span class="channel-name">{{ ch.label || ch.name || ch.key }}</span>
            <el-tag :type="ch.configured ? 'success' : 'info'" size="small" style="margin-left:8px">
              {{ ch.configured ? '已配置' : '未配置' }}
            </el-tag>
            <el-button
              v-if="ch.configured"
              link
              type="primary"
              style="margin-left:auto"
              @click="testChannel(ch.key || ch.name)"
              :loading="testingChannel === (ch.key || ch.name)"
            >
              测试
            </el-button>
          </div>
          <el-empty v-if="!channels.length" description="暂无通知渠道" />
        </el-card>
      </el-col>
    </el-row>

    <el-dialog v-model="createDialogVisible" :title="editingRule ? '编辑规则' : '新建规则'" width="500px">
      <el-form :model="ruleForm" label-width="80px">
        <el-form-item label="股票代码">
          <el-autocomplete
            v-model="ruleForm.code"
            :fetch-suggestions="queryStocks"
            placeholder="输入代码或名称搜索"
            style="width:100%"
            @select="onStockSelect"
          >
            <template #default="{ item }">
              <span style="margin-right:8px">{{ item.code }}</span>
              <span style="color:var(--el-text-color-secondary)">{{ item.name }}</span>
            </template>
          </el-autocomplete>
        </el-form-item>
        <el-form-item label="规则名称">
          <el-input v-model="ruleForm.name" placeholder="规则名称" />
        </el-form-item>
        <el-form-item label="规则类型">
          <el-select v-model="ruleForm.rule_type" style="width:100%">
            <el-option label="价格突破" value="price_breakout" />
            <el-option label="涨跌幅" value="change_percent" />
            <el-option label="成交量" value="volume" />
            <el-option label="技术指标" value="technical" />
            <el-option label="自定义" value="custom" />
          </el-select>
        </el-form-item>
        <el-form-item label="条件(JSON)">
          <el-input v-model="ruleForm.conditionStr" type="textarea" :rows="4" placeholder='{"field":"price","op":">","value":100}' />
        </el-form-item>
        <el-form-item v-if="!editingRule">
          <el-button @click="testNewRule" :loading="testLoading">测试规则</el-button>
        </el-form-item>
      </el-form>
      <div v-if="testResult" style="margin-top:12px">
        <el-alert :title="testResult.triggered ? '条件已触发' : '条件未触发'" :type="testResult.triggered ? 'success' : 'info'" show-icon />
      </div>
      <template #footer>
        <el-button @click="createDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="submitRule">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { alertApi, stockSearch } from '@/api/alert'
import { notificationApi } from '@/api/notification'

const rules = ref<any[]>([])
const triggers = ref<any[]>([])
const channels = ref<any[]>([])
const testingChannel = ref('')

const createDialogVisible = ref(false)
const editingRule = ref<Record<string, any> | null>(null)
const ruleForm = ref({
  code: '',
  name: '',
  rule_type: 'price_breakout',
  conditionStr: '{}',
})
const submitting = ref(false)
const testLoading = ref(false)
const testResult = ref<Record<string, any> | null>(null)

async function queryStocks(queryString: string, cb: (results: any[]) => void) {
  if (!queryString || queryString.length < 1) {
    cb([])
    return
  }
  try {
    const res: any = await stockSearch(queryString, 10)
    const list = (res.data || res || []) as any[]
    cb(list.map((s: any) => ({ ...s, value: s.code })))
  } catch {
    cb([])
  }
}

function onStockSelect(item: any) {
  ruleForm.value.code = item.code
  ruleForm.value.name = ruleForm.value.name || item.name
}

async function loadRules() {
  try {
    const res: any = await alertApi.rules()
    rules.value = res.data || []
  } catch { /* ignore */ }
}

async function loadTriggers() {
  try {
    const res: any = await alertApi.triggers(50)
    triggers.value = res.data || []
  } catch { /* ignore */ }
}

async function loadChannels() {
  try {
    const res: any = await notificationApi.channels()
    channels.value = res.data || []
  } catch { /* ignore */ }
}

async function toggleRule(row: Record<string, any>, enabled: boolean) {
  try {
    if (enabled) {
      await alertApi.ruleEnable(row.id)
    } else {
      await alertApi.ruleDisable(row.id)
    }
    row.enabled = enabled
    ElMessage.success(enabled ? '规则已启用' : '规则已禁用')
  } catch {
    ElMessage.error('操作失败')
  }
}

async function deleteRule(row: Record<string, any>) {
  try {
    await alertApi.ruleDelete(row.id)
    ElMessage.success('规则已删除')
    loadRules()
  } catch {
    ElMessage.error('删除失败')
  }
}

function openCreateDialog() {
  editingRule.value = null
  ruleForm.value = { code: '', name: '', rule_type: 'price_breakout', conditionStr: '{}' }
  testResult.value = null
  createDialogVisible.value = true
}

function editRule(row: Record<string, any>) {
  editingRule.value = row
  ruleForm.value = {
    code: row.code || '',
    name: row.name || '',
    rule_type: row.rule_type || row.rule_type || 'price_breakout',
    conditionStr: JSON.stringify(row.condition || {}, null, 2),
  }
  testResult.value = null
  createDialogVisible.value = true
}

async function testNewRule() {
  testLoading.value = true
  try {
    const condition = JSON.parse(ruleForm.value.conditionStr)
    const res: any = await alertApi.ruleTest(ruleForm.value.code, condition)
    testResult.value = res.data || { triggered: false }
  } catch (e) {
    ElMessage.error('测试失败，请检查JSON格式')
  } finally {
    testLoading.value = false
  }
}

async function testRule(row: Record<string, any>) {
  try {
    const res: any = await alertApi.ruleTest(row.code, row.condition || {})
    const data = res.data || {}
    ElMessage.success(data.triggered ? '条件已触发' : '条件未触发')
  } catch {
    ElMessage.error('测试失败')
  }
}

async function submitRule() {
  let condition: Record<string, any>
  try {
    condition = JSON.parse(ruleForm.value.conditionStr)
  } catch {
    ElMessage.error('条件JSON格式错误')
    return
  }
  submitting.value = true
  try {
    if (editingRule.value) {
      await alertApi.ruleUpdate(editingRule.value.id, condition)
      ElMessage.success('规则已更新')
    } else {
      await alertApi.ruleCreate({
        code: ruleForm.value.code,
        rule_type: ruleForm.value.rule_type,
        name: ruleForm.value.name,
        condition,
      })
      ElMessage.success('规则已创建')
    }
    createDialogVisible.value = false
    loadRules()
  } catch {
    ElMessage.error(editingRule.value ? '更新失败' : '创建失败')
  } finally {
    submitting.value = false
  }
}

async function testChannel(channel: string) {
  testingChannel.value = channel
  try {
    await notificationApi.test(channel)
    ElMessage.success(`测试消息已发送至 ${channel}`)
  } catch {
    ElMessage.error(`测试消息发送失败`)
  } finally {
    testingChannel.value = ''
  }
}

onMounted(() => {
  loadRules()
  loadTriggers()
  loadChannels()
})
</script>

<style scoped lang="scss">
.channel-item {
  display: flex;
  align-items: center;
  padding: 8px 0;
  border-bottom: 1px solid var(--el-border-color-lighter);
  &:last-child {
    border-bottom: none;
  }
}
.channel-name {
  font-size: 14px;
}
</style>
