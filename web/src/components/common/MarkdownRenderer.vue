<template>
  <div class="markdown-renderer" v-html="rendered" />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import MarkdownIt from 'markdown-it'

const props = defineProps<{ content: string }>()

const md = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: true,
  typographer: false,
})

const rendered = computed(() => {
  return md.render(props.content || '')
})
</script>

<style scoped lang="scss">
.markdown-renderer {
  line-height: 1.75;
  font-size: 14px;
  word-break: break-word;

  :deep(h1), :deep(h2), :deep(h3), :deep(h4) {
    margin-top: 16px;
    margin-bottom: 8px;
    font-weight: 600;
    line-height: 1.4;
  }
  :deep(h1) { font-size: 1.3em; }
  :deep(h2) { font-size: 1.15em; }
  :deep(h3) { font-size: 1.05em; }
  :deep(h4) { font-size: 1em; }

  :deep(p) {
    margin: 6px 0;
  }

  :deep(strong) {
    font-weight: 600;
    color: var(--el-text-color-primary);
  }

  :deep(em) {
    font-style: italic;
  }

  :deep(ul), :deep(ol) {
    padding-left: 20px;
    margin: 6px 0;
  }
  :deep(li) {
    margin: 2px 0;
  }

  :deep(code) {
    background: var(--el-fill-color-light);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 13px;
    font-family: 'Menlo', 'Monaco', 'Courier New', monospace;
    color: var(--el-color-danger-light-3);
  }

  :deep(pre) {
    background: var(--el-fill-color-lighter);
    border-radius: 6px;
    padding: 12px 16px;
    overflow-x: auto;
    margin: 8px 0;

    code {
      background: none;
      padding: 0;
      border-radius: 0;
      font-size: 13px;
      color: var(--el-text-color-primary);
    }
  }

  :deep(blockquote) {
    border-left: 4px solid var(--el-color-primary-light-5);
    padding: 4px 12px;
    margin: 8px 0;
    color: var(--el-text-color-secondary);
    background: var(--el-color-primary-light-9);
    border-radius: 0 4px 4px 0;
  }

  :deep(table) {
    width: 100%;
    border-collapse: collapse;
    margin: 8px 0;
    font-size: 13px;

    th, td {
      border: 1px solid var(--el-border-color-lighter);
      padding: 6px 10px;
      text-align: left;
    }

    th {
      background: var(--el-fill-color-lighter);
      font-weight: 600;
    }

    tr:nth-child(even) {
      background: var(--el-fill-color-lighter);
    }
  }

  :deep(hr) {
    border: none;
    border-top: 1px solid var(--el-border-color-lighter);
    margin: 12px 0;
  }

  :deep(a) {
    color: var(--el-color-primary);
    text-decoration: none;
    &:hover {
      text-decoration: underline;
    }
  }

  :deep(.up), :deep(.pnl-up) {
    color: #f56c6c;
    font-weight: 500;
  }
  :deep(.down), :deep(.pnl-down) {
    color: #67c23a;
    font-weight: 500;
  }
}
</style>
