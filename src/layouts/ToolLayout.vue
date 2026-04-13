<script setup lang="ts">
import { useMagicKeys, watchArray } from "@vueuse/core";
import { computed } from "vue";
import { useRouter } from "vue-router";

const props = withDefaults(defineProps<{ query?: string; preventEscape?: boolean }>(), {
  query: "",
  preventEscape: false,
});

const emit = defineEmits<{
  "submit:query": [];
  "update:query": [value: string];
  action: [value: "enter" | "escape"];
}>();

const { current } = useMagicKeys();
const keys = computed(() => Array.from(current));
const router = useRouter();

watchArray(keys, (value) => {
  if (value.includes("escape")) {
    emit("action", "escape");
    if (!props.preventEscape) router.back();
  } else if (value.includes("enter")) {
    emit("action", "enter");
  }
});
</script>

<template>
  <main class="flex flex-col w-screen h-screen bg-dark-400 text-white overflow-hidden" tabindex="0">
    <div class="border-b border-white/10 px-5 py-3 flex items-center gap-3 shrink-0">
      <slot name="header">
        <input
          :value="query"
          @input="(e) => emit('update:query', (e.target as HTMLInputElement).value)"
          @keydown.enter="emit('submit:query')"
          placeholder="Type a command or input..."
          class="w-full bg-transparent outline-none text-white placeholder-light-400 text-lg font-medium"
        />
      </slot>
    </div>

    <div class="flex-1 min-h-0">
      <slot />
    </div>

    <div class="border-t border-white/10 px-5 py-3 flex items-center justify-between shrink-0">
      <slot name="footer">
        <span class="text-xs text-white/40">ESC to go back</span>
        <button
          class="bg-white/10 px-2 py-1 rounded text-xs text-white hover:bg-white/20"
          @click="emit('action', 'enter')"
        >
          Enter ↵
        </button>
      </slot>
    </div>
  </main>
</template>