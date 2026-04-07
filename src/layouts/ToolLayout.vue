<script setup lang="ts">
import { useMagicKeys, watchArray } from "@vueuse/core";
import { computed } from "vue";
import { useRouter } from "vue-router";

defineProps<{
  query: string;
}>();

const actionKeys = ["enter", "escape"] as const;
type ActionKey = (typeof actionKeys)[number];

const emit = defineEmits<{
  "submit:query": [];
  "update:query": [value: string];
  action: [value: ActionKey];
}>();

const { current } = useMagicKeys();
const keys = computed(() => Array.from(current));

const router = useRouter();

watchArray(keys, (value) => {
  actionKeys.forEach((key) => {
    if (value.includes(key)) {
      emit("action", key);
      if (key === "escape") router.back();
    }
  });
});
</script>

<template>
  <main class="flex flex-col w-screen h-screen bg-[#1e1e1e]/95 text-white" tabindex="0">
    <!-- Top Search -->
    <div class="px-5 py-4 border-b border-white/10 flex items-center gap-3">
      <input
        :value="query"
        @input="(event) => emit('update:query', event.target.value)"
        @keydown.enter="emit('submit:query')"
        placeholder="Type a command or input..."
        class="w-full bg-transparent outline-none text-white placeholder:text-gray-400 text-lg font-medium"
      />
    </div>
    <!-- Dynamic Content -->
    <div class="flex-1 overflow-y-auto p-5">
      <slot />
    </div>
    <!-- Footer -->
    <div
      class="px-5 py-3 border-t border-white/10 flex justify-between items-center text-xs text-gray-400"
    >
      <span>ESC to go back</span>
      <span class="bg-white/10 px-2 py-1 rounded-md text-white text-xs"> Enter ↵ </span>
    </div>
  </main>
</template>