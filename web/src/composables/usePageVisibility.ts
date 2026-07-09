import { ref, onMounted, onUnmounted } from 'vue'

export function usePageVisibility() {
  const isVisible = ref(!document.hidden)

  function onVisibilityChange() {
    isVisible.value = !document.hidden
  }

  onMounted(() => document.addEventListener('visibilitychange', onVisibilityChange))
  onUnmounted(() => document.removeEventListener('visibilitychange', onVisibilityChange))

  return { isVisible }
}
