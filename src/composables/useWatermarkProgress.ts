import { ref, onUnmounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export interface ProgressState {
  total: number
  processed: number
  currentFile: string
}

export function useWatermarkProgress() {
  const state = ref<ProgressState>({ total: 0, processed: 0, currentFile: '' })
  let unlisten: UnlistenFn | null = null

  async function startListening() {
    unlisten = await listen<string>('sidecar:event', (e) => {
      try {
        const data = JSON.parse(e.payload)
        if (data.type === 'total') {
          state.value.total = data.total
        } else if (data.type === 'progress') {
          state.value.processed = data.processed
          state.value.total = data.total
          state.value.currentFile = data.file
        }
      } catch {
        // malformed — ignore
      }
    })
  }

  function stopListening() {
    unlisten?.()
    unlisten = null
  }

  onUnmounted(stopListening)

  return { state, startListening, stopListening }
}