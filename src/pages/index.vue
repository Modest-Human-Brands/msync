<script setup lang="ts">
import { ref, computed } from "vue";
import { useRouter } from "vue-router";
import ToolLayout from "../layouts/ToolLayout.vue";

import IconFolder from "~icons/app/folder";
import IconLayers from "~icons/app/layers";
import IconDownscale from "~icons/app/downscale";
import IconBroadcast from "~icons/app/downscale";

const router = useRouter();
const query = ref("");

const tools = [
  {
    id: "stream",
    title: "Stream media",
    description: "Preview USB video stream and broadcast to a remote via SRT",
    icon: IconBroadcast,
  },
  {
    id: "reorganize",
    title: "Reorganize Files",
    description: "Organize files based on spec.json",
    icon: IconFolder,
  },
  {
    id: "overlay",
    title: "Add Overlay / Watermark",
    description: "Apply watermark overlay to selected assets",
    icon: IconLayers,
  },
  {
    id: "sync",
    title: "Sync media",
    description: "Sync files across devices/cloud",
    icon: IconDownscale,
  },
];

const filtered = computed(() =>
  tools.filter((t) => (t.title + t.description).toLowerCase().includes(query.value.toLowerCase())),
);

function onSubmit() {
  if (filtered.value[0]) {
    router.push(`/${filtered.value[0].id}`);
  }
}
</script>

<template>
  <ToolLayout v-model:query="query" @submit:query="onSubmit">
    <div>
      <div
        v-for="tool in filtered"
        :key="tool.id"
        @click="router.push(`/${tool.id}`)"
        class="px-3 py-3 flex items-start gap-3 hover:bg-white/5 rounded cursor-pointer"
      >
        <component :is="tool.icon" class="size-6 text-gray-300" />

        <div>
          <div class="text-sm font-medium">{{ tool.title }}</div>
          <div class="text-xs text-gray-400">{{ tool.description }}</div>
        </div>
      </div>

      <div v-if="filtered.length === 0" class="text-center text-gray-500 py-6">No tool found</div>
    </div>
  </ToolLayout>
</template>