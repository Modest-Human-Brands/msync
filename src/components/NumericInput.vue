<script setup lang="ts">
defineProps<{
  label: string;
  modelValue: number;
  unit?: string;
  min?: number;
  max?: number;
  step?: number;
  readonly?: boolean;
}>();

defineEmits<{ "update:modelValue": [number] }>();
</script>

<template>
  <div
    class="flex items-center gap-1 bg-black/60 border border-white/10 rounded px-2 py-0.5"
    :class="readonly ? 'opacity-50' : ''"
  >
    <span class="text-2xs text-white/30 w-3 shrink-0 select-none font-medium">{{ label }}</span>
    <input
      type="number"
      :value="modelValue"
      :min="min"
      :max="max"
      :step="step ?? 1"
      :readonly="readonly"
      class="w-full bg-transparent text-xs text-white font-medium outline-none text-right no-spinner"
      :class="readonly ? 'cursor-not-allowed' : ''"
      @input="(e) => $emit('update:modelValue', Number((e.target as HTMLInputElement).value))"
    />
    <span v-if="unit" class="text-2xs text-white/20 shrink-0 select-none">{{ unit }}</span>
  </div>
</template>

<style scoped>
.no-spinner::-webkit-outer-spin-button,
.no-spinner::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}

.no-spinner {
  -moz-appearance: textfield;
}
</style>