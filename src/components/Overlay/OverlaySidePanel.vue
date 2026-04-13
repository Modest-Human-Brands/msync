<script setup lang="ts">
import { computed } from 'vue'
import NumericInput from '../NumericInput.vue'

const hAlign = defineModel<'left' | 'center' | 'right'>('hAlign', { required: true })
const vAlign = defineModel<'top' | 'middle' | 'bottom'>('vAlign', { required: true })
const wmScale = defineModel<number>('wmScale', { required: true })
const wmOpacity = defineModel<number>('wmOpacity', { required: true })
const heightLocked = defineModel<boolean>('heightLocked', { required: true })
const wmManualHeight = defineModel<number>('wmManualHeight', { required: true })

const props = defineProps<{
  watermarkName: string
  watermarkSrc: string
  watermarkNaturalSize: { w: number; h: number }
  positionX: number
  positionY: number
  wmWidthPx: number
  wmHeightPx: number
  canvasWidth: number
}>()

const emit = defineEmits<{ 'browse-watermark': [] }>()

const opacityPercent = computed({
  get: () => Math.round(wmOpacity.value * 100),
  set: (v) => { wmOpacity.value = v / 100 },
})

function onWidthInput(val: number) {
  wmScale.value = Math.max(1, Math.round((val / props.canvasWidth) * 100))
}

function toggleLock() {
  if (heightLocked.value) wmManualHeight.value = props.wmHeightPx
  heightLocked.value = !heightLocked.value
}

const H_ALIGNS: { value: 'left' | 'center' | 'right'; rect: [number, number, number, number] }[] = [
  { value: 'left', rect: [2, 5, 5, 6] },
  { value: 'center', rect: [5.5, 5, 5, 6] },
  { value: 'right', rect: [9, 5, 5, 6] },
]

const V_ALIGNS: { value: 'top' | 'middle' | 'bottom'; rect: [number, number, number, number] }[] = [
  { value: 'top', rect: [5, 2, 6, 5] },
  { value: 'middle', rect: [5, 5.5, 6, 5] },
  { value: 'bottom', rect: [5, 9, 6, 5] },
]
</script>

<template>
  <aside class="w-56 shrink-0 flex flex-col border-l border-white/10 bg-dark-400 overflow-y-auto">
    <div class="flex flex-col gap-5 px-4 py-4">
      <section>
        <button
          class="w-full flex items-center gap-3 bg-black/30 border border-white/10 hover:border-white/20 rounded px-3 py-2.5 transition-colors text-left"
          @click="emit('browse-watermark')">
          <div
            class="w-10 h-10 bg-black/50 border border-white/10 flex items-center justify-center shrink-0 rounded overflow-hidden">
            <img v-if="watermarkSrc" :src="watermarkSrc" class="w-full h-full object-contain" />
            <svg v-else width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"
              class="text-white/20">
              <path
                d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" />
              <polyline points="3.27 6.96 12 12.01 20.73 6.96" />
              <line x1="12" y1="22.08" x2="12" y2="12" />
            </svg>
          </div>
          <div class="min-w-0">
            <p class="text-xs text-white/70 truncate">{{ watermarkName || 'Select watermark' }}</p>
            <p v-if="watermarkNaturalSize.w && watermarkNaturalSize.h" class="text-2xs text-white/30 mt-0.5">
              {{ watermarkNaturalSize.w }} × {{ watermarkNaturalSize.h }} PX
            </p>
          </div>
        </button>
      </section>

      <section>
        <h3 class="text-2xs font-semibold text-white/30 tracking-widest uppercase mb-2">Position</h3>
        <div class="flex items-center gap-2 mb-3">
          <div class="flex bg-black/40 border border-white/10 rounded overflow-hidden">
            <button v-for="a in H_ALIGNS" :key="a.value" class="p-1.5 transition-colors"
              :class="hAlign === a.value ? 'bg-primary-500/20 text-primary-500' : 'text-white/30 hover:text-white/60 hover:bg-white/5'"
              @click="hAlign = a.value">
              <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                <rect x="0.5" y="0.5" width="15" height="15" rx="1.5" fill="none" stroke="currentColor"
                  stroke-width="0.8" opacity="0.3" />
                <rect :x="a.rect[0]" :y="a.rect[1]" :width="a.rect[2]" :height="a.rect[3]" rx="0.5" />
              </svg>
            </button>
          </div>
          <div class="flex bg-black/40 border border-white/10 rounded overflow-hidden">
            <button v-for="a in V_ALIGNS" :key="a.value" class="p-1.5 transition-colors"
              :class="vAlign === a.value ? 'bg-primary-500/20 text-primary-500' : 'text-white/30 hover:text-white/60 hover:bg-white/5'"
              @click="vAlign = a.value">
              <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                <rect x="0.5" y="0.5" width="15" height="15" rx="1.5" fill="none" stroke="currentColor"
                  stroke-width="0.8" opacity="0.3" />
                <rect :x="a.rect[0]" :y="a.rect[1]" :width="a.rect[2]" :height="a.rect[3]" rx="0.5" />
              </svg>
            </button>
          </div>
        </div>
        <div class="grid grid-cols-2 gap-2">
          <NumericInput label="X" :model-value="positionX" :readonly="true" />
          <NumericInput label="Y" :model-value="positionY" :readonly="true" />
        </div>
      </section>

      <section>
        <h3 class="text-2xs font-semibold text-white/30 tracking-widest uppercase mb-2">Transform</h3>
        <div class="flex flex-col gap-2">
          <div class="flex items-center gap-1.5">
            <NumericInput label="W" :model-value="wmWidthPx" :min="1" class="flex-1"
              @update:model-value="onWidthInput" />
            <button class="p-1 rounded transition-colors shrink-0"
              :class="heightLocked ? 'text-primary-500' : 'text-white/20 hover:text-white/50'" @click="toggleLock">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                stroke-linecap="round" stroke-linejoin="round">
                <rect x="3" y="11" width="18" height="11" rx="2" />
                <path v-if="heightLocked" d="M7 11V7a5 5 0 0 1 10 0v4" />
                <path v-else d="M7 11V7a5 5 0 0 1 9.9-1" />
              </svg>
            </button>
            <NumericInput label="H" :model-value="wmHeightPx" :readonly="heightLocked" :min="1" class="flex-1"
              @update:model-value="wmManualHeight = $event" />
          </div>
          <div class="grid grid-cols-2 gap-2">
            <NumericInput label="S" v-model="wmScale" unit="%" :min="1" :max="100" />
            <NumericInput label="O" :model-value="opacityPercent" unit="%" :min="0" :max="100"
              @update:model-value="opacityPercent = $event" />
          </div>
        </div>
      </section>
    </div>
  </aside>
</template>