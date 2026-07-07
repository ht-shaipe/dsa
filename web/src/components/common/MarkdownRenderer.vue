<template>
  <div class="markdown-renderer" v-html="rendered" />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import MarkdownIt from 'markdown-it'

const props = defineProps<{ content: string }>()

const md = new MarkdownIt({ html: false, linkify: true, breaks: true })

const rendered = computed(() => {
  return md.render(props.content || '')
})
</script>

<style scoped lang="scss">
.markdown-renderer {
  line-height: 1.7;
  font-size: 14px;
  :deep(h1), :deep(h2), :deep(h3) {
    margin-top: 16px;
    margin-bottom: 8px;
  }
  :deep(p) {
    margin: 8px 0;
  }
  :deep(ul), :deep(ol) {
    padding-left: 20px;
  }
  :deep(code) {
    background: var(--el-fill-color-light);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 13px;
  }
  :deep(blockquote) {
    border-left: 4px solid var(--el-color-primary);
    padding-left: 12px;
    color: var(--el-text-color-secondary);
  }
}
</style>
