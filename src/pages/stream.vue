<script setup lang="ts">
import { ref, watchEffect, onMounted, onUnmounted } from 'vue'
import { useUserMedia } from '@vueuse/core'
import ToolLayout from "../layouts/ToolLayout.vue";

const videoElement = ref(null)

const { stream, start, stop } = useUserMedia({
  constraints: {
    video: {
      facingMode: 'environment',
      width: { ideal: 1920 },
      height: { ideal: 1080 }
    },
    audio: false
  }
})

watchEffect(() => {
  if (videoElement.value && stream.value) {
    videoElement.value.srcObject = stream.value
  }
})

onMounted(() => {
  start()
})

onUnmounted(() => {
  stop()
})
</script>

<template>
  <ToolLayout>
    <template #header>
      <div class="flex items-center gap-2 min-w-0">
        <span class="text-white/50 shrink-0 uppercase text-xs tracking-wider">Stream Media</span>
      </div>
    </template>
    <main class="fixed inset-0 w-screen h-screen bg-black flex items-center justify-center overflow-hidden">
      <video ref="videoElement" class="w-full h-full object-cover" autoplay playsinline muted controls />
      <div v-if="!stream" class="absolute text-white/50 font-mono tracking-widest animate-pulse">
        INITIALIZING CAMERA...
      </div>
    </main>
  </ToolLayout>
</template>