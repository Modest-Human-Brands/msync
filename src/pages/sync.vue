<script setup lang="ts">
import { ref, computed } from "vue";
import { open } from "@tauri-apps/plugin-dialog";

import IconSync from "~icons/app/downscale";
import ToolLayout from "../layouts/ToolLayout.vue";
import { MediaItem } from "../types";

const query = ref("");
const mediaFiles = ref<MediaItem[]>([]);
const selectedIndex = ref(0);

const selectedFile = computed(() => mediaFiles.value[selectedIndex.value]);
const hasFiles = computed(() => mediaFiles.value.length > 0);

const isImage = (file?: MediaItem) =>
  !!file &&
  (file.type.startsWith("image") ||
    ["png", "jpg", "jpeg", "webp", "gif"].includes(file.type.toLowerCase()));
const formatSize = (size: number) => (size / 1024).toFixed(2);

async function onBrowse() {
  const selected = await open({ multiple: false, directory: true });
  if (!selected) return;

  const dir = Array.isArray(selected) ? selected[0] : selected;
  mediaFiles.value = await listFilesRecursively(dir);
  selectedIndex.value = 0;
}

function handleDrop(e: DragEvent) {
  e.preventDefault();
  mediaFiles.value = Array.from(e.dataTransfer?.files || []).map((f) => ({
    path: f.name,
    name: f.name,
    type: f.type,
    size: f.size,
  }));
  selectedIndex.value = 0;
}

function onSubmit() {
  console.log("Files:", mediaFiles.value);
}
</script>

<template>
  <ToolLayout v-model:query="query" @submit:query="onSubmit" @action="(v) => v === 'enter' && onBrowse()">
    <div v-if="!hasFiles" @dragover.prevent @drop="handleDrop"
      class="h-full flex flex-col items-center justify-center text-center text-gray-400">
      <div class="flex flex-col items-center gap-3">
        <IconSync class="size-10 opacity-60" />
        <div class="text-sm font-medium">Drop files here to import</div>
        <div class="text-xs text-gray-500">or press Enter to browse</div>
        <button @click="onBrowse"
          class="mt-2 px-4 py-2 rounded-lg bg-white/8 hover:bg-white/12 border border-white/10 text-xs text-white/80 transition">
          Browse folder
        </button>
      </div>
    </div>

    <div v-else class="flex h-full gap-4">
      <div class="w-[45%] overflow-y-auto p-1">
        <div class="grid grid-cols-3 gap-2">
          <div v-for="(item, i) in mediaFiles" :key="item.path" @click="selectedIndex = i"
            class="relative group cursor-pointer rounded-md overflow-hidden bg-white/5"
            :class="{ 'ring-2 ring-white/40': i === selectedIndex }">
            <img v-if="isImage(item)" :src="item.url" class="w-full h-24 object-cover" />
            <div v-else class="w-full h-24 flex items-center justify-center text-xs text-gray-500">
              {{ item.type || "FILE" }}
            </div>

            <div
              class="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 transition flex flex-col justify-end p-2">
              <span class="text-xs text-white truncate">{{ item.name }}</span>
            </div>
          </div>
        </div>
      </div>

      <div class="flex-1 flex flex-col gap-4">
        <div class="bg-white/5 rounded-xl flex items-center justify-center h-80">
          <img v-if="isImage(selectedFile)" :src="selectedFile.url" class="max-h-full max-w-full object-contain" />
          <span v-else class="text-gray-500 text-sm">No preview available</span>
        </div>

        <div class="bg-white/5 rounded-xl p-4 space-y-3 text-sm">
          <div class="flex items-center gap-2">
            <span class="text-gray-400 text-xs">📄</span>
            <span class="text-white truncate">{{ selectedFile?.name }}</span>
          </div>
          <div class="flex items-center gap-2">
            <span class="text-gray-400 text-xs">🧩</span>
            <span class="text-gray-300">{{ selectedFile?.type || "unknown" }}</span>
          </div>
          <div class="flex items-center gap-2">
            <span class="text-gray-400 text-xs">📦</span>
            <span class="text-gray-300">{{ selectedFile ? formatSize(selectedFile.size) : "0" }} KB</span>
          </div>
          <div v-if="isImage(selectedFile)" class="flex items-center gap-2">
            <span class="text-gray-400 text-xs">🖼️</span>
            <span class="text-gray-300">Image preview</span>
          </div>
        </div>
      </div>
    </div>
  </ToolLayout>
</template>