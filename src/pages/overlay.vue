<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { convertFileSrc, invoke } from "@tauri-apps/api/core"; // Added convertFileSrc
import { useIntervalFn } from "@vueuse/core";
import IconDownscale from "~icons/app/downscale";
import ToolLayout from "../layouts/ToolLayout.vue";
import { useWatermarkProgress } from "../composables/useWatermarkProgress";
import { MediaFile, listFilesRecursively } from "../lib/sync";

type Position = "top-left" | "top-right" | "bottom-left" | "bottom-right" | "center";
type Step = "configure" | "processing" | "done";

const query = ref("");
const step = ref<Step>("configure");

const srcDir = ref("");
const destDir = ref("");
const watermarkPath = ref("");
const position = ref<Position>("bottom-right");
const wmSize = ref(20);
const wmOpacity = ref(0.5);

const mediaFiles = ref<MediaFile[]>([]);
const selectedIndex = ref(0);
const loadingMedia = ref(false);

const processing = ref({
  total: 0,
  processed: 0,
  currentFile: "",
  startedAt: 0,
  errors: [] as string[],
});
const result = ref<{ total: number; errors: number; elapsedMs: number } | null>(null);
const now = ref(Date.now());

const { state: progressState, startListening, stopListening } = useWatermarkProgress();
const { pause: pauseTimer, resume: resumeTimer } = useIntervalFn(
  () => {
    now.value = Date.now();
  },
  200,
  { immediate: false },
);

const selectedMedia = computed(() => mediaFiles.value[selectedIndex.value]);
const hasSelection = computed(() => mediaFiles.value.length > 0);
const isValid = computed(
  () =>
    srcDir.value.trim() !== "" && destDir.value.trim() !== "" && watermarkPath.value.trim() !== "",
);

const pct = computed(() =>
  progressState.value.total > 0
    ? Math.round((progressState.value.processed / progressState.value.total) * 100)
    : 0,
);

const elapsedLabel = computed(() => {
  if (!processing.value.startedAt) return "0s";
  const s = (now.value - processing.value.startedAt) / 1000;
  return s < 60 ? `${s.toFixed(1)}s` : `${Math.floor(s / 60)}m ${Math.floor(s % 60)}s`;
});

// BUG FIX: Watermark needs convertFileSrc to be visible in the browser preview
const watermarkSrc = computed(() =>
  watermarkPath.value ? convertFileSrc(watermarkPath.value) : "",
);

const watermarkStyle = computed(() => {
  const base: Record<string, string> = {
    position: "absolute",
    width: `${wmSize.value}%`,
    opacity: String(wmOpacity.value),
    pointerEvents: "none",
    userSelect: "none",
  };
  const transforms: Record<Position, Record<string, string>> = {
    "top-left": { top: "12px", left: "12px", transform: "none" },
    "top-right": { top: "12px", right: "12px", transform: "none" },
    "bottom-left": { bottom: "12px", left: "12px", transform: "none" },
    "bottom-right": { bottom: "12px", right: "12px", transform: "none" },
    center: { top: "50%", left: "50%", transform: "translate(-50%, -50%)" },
  };
  return { ...base, ...transforms[position.value] };
});

const isImageFile = (item?: Pick<MediaFile, "type" | "name">) =>
  !!item && (item.type.startsWith("image/") || /\.(png|jpe?g|webp|gif|bmp|svg)$/i.test(item.name));
const formatSize = (size?: number) => (size ? (size / 1024).toFixed(1) : "0");

// BUG FIX: Unified loading logic that attaches the necessary .url for <img> src bindings
async function loadMediaFromFolder(folder: string) {
  loadingMedia.value = true;
  try {
    const items = await listFilesRecursively(folder);
    mediaFiles.value = items.map((item) => ({
      ...item,
      url: convertFileSrc(item.path),
    }));
    selectedIndex.value = 0;
  } catch (error) {
    console.error("Failed to list media files:", error);
    mediaFiles.value = [];
  } finally {
    loadingMedia.value = false;
  }
}

// BUG FIX: Updated to set srcDir.value. This fixes the "isValid" check and triggers the watcher.
async function onBrowse() {
  const selected = await open({ multiple: false, directory: true });
  if (!selected) return;

  const dir = Array.isArray(selected) ? selected[0] : selected;
  srcDir.value = dir;

  // Auto-set a default output folder if one isn't selected
  if (!destDir.value) destDir.value = `${dir}_watermarked`;
}

// BUG FIX: Implemented the missing watermark browsing function
async function browseWatermark() {
  const selected = await open({
    multiple: false,
    filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "webp", "svg"] }],
  });
  if (selected) {
    watermarkPath.value = Array.isArray(selected) ? selected[0] : selected;
  }
}

async function browseOutput() {
  const selected = await open({ multiple: false, directory: true });
  if (selected) {
    destDir.value = Array.isArray(selected) ? selected[0] : selected;
  }
}

watch(srcDir, async (value) => {
  if (!value) {
    mediaFiles.value = [];
    selectedIndex.value = 0;
    return;
  }
  await loadMediaFromFolder(value);
});

async function run() {
  if (!isValid.value || step.value !== "configure") return;

  step.value = "processing";
  processing.value = { total: 0, processed: 0, currentFile: "", startedAt: Date.now(), errors: [] };
  progressState.value = { total: 0, processed: 0, currentFile: "" };

  now.value = Date.now();
  resumeTimer();
  await startListening();

  const startedAt = Date.now();

  try {
    const res = await invoke<{ total: number; errors: string[] }>("add_watermarks", {
      srcDir: srcDir.value,
      destDir: destDir.value,
      watermarkPath: watermarkPath.value,
      position: position.value,
      size: wmSize.value,
      opacity: wmOpacity.value,
    });

    // BUG FIX: Capturing error strings from the backend result
    processing.value.errors = res.errors;
    result.value = {
      total: res.total,
      errors: res.errors.length,
      elapsedMs: Date.now() - startedAt,
    };
  } catch (err) {
    console.error("Watermark process failed:", err);
    result.value = {
      total: progressState.value.processed,
      errors: 1,
      elapsedMs: Date.now() - startedAt,
    };
  }

  stopListening();
  pauseTimer();
  step.value = "done";
}

function onSubmit() {
  if (step.value === "configure") run();
  else if (step.value === "done") step.value = "configure";
}

function formatMs(ms: number) {
  const s = ms / 1000;
  return s < 60 ? `${s.toFixed(2)}s` : `${Math.floor(s / 60)}m ${(s % 60).toFixed(0)}s`;
}

const POSITIONS: { value: Position; col: number; row: number; label: string }[] = [
  { value: "top-left", col: 1, row: 1, label: "TL" },
  { value: "top-right", col: 3, row: 1, label: "TR" },
  { value: "center", col: 2, row: 2, label: "·" },
  { value: "bottom-left", col: 1, row: 3, label: "BL" },
  { value: "bottom-right", col: 3, row: 3, label: "BR" },
];
</script>
<template>
  <ToolLayout
    v-model:query="query"
    @submit:query="onSubmit"
    @action="(v) => v === 'enter' && onBrowse()"
  >
    <div
      v-if="step === 'configure' && !hasSelection"
      @dragover.prevent
      class="h-full flex flex-col items-center justify-center text-center text-gray-400"
    >
      <div class="flex flex-col items-center gap-3">
        <IconDownscale class="size-10 opacity-60" />
        <div class="text-sm font-medium">Choose a source folder to preview all media</div>
        <div class="text-xs text-gray-500">Press Enter or click browse to load images</div>
        <button
          class="mt-2 px-4 py-2 rounded-lg bg-white/8 hover:bg-white/12 border border-white/10 text-xs text-white/80 transition"
          @click="onBrowse"
        >
          Browse folder
        </button>
      </div>
    </div>

    <div v-else-if="step === 'configure'" class="flex h-full gap-4">
      <div class="w-[45%] min-w-0 overflow-y-auto p-1 scrollbar-hidden">
        <div class="mb-3 flex items-center justify-between gap-2">
          <div class="min-w-0">
            <div class="text-2xs uppercase tracking-widest text-gray-500">Source media</div>
            <div class="text-xs text-gray-400 truncate">{{ srcDir || "No folder selected" }}</div>
          </div>
          <button
            class="shrink-0 px-2.5 py-1.5 rounded-md bg-white/8 hover:bg-white/12 border border-white/10 text-2xs text-gray-300 hover:text-white transition"
            @click="onBrowse"
          >
            Browse
          </button>
        </div>

        <div
          v-if="loadingMedia"
          class="h-80 rounded-xl bg-white/5 border border-white/8 flex items-center justify-center text-xs text-gray-500"
        >
          Loading media…
        </div>

        <div v-else class="grid grid-cols-3 gap-2">
          <div
            v-for="(item, i) in mediaFiles"
            :key="item.path"
            @click="selectedIndex = i"
            class="relative group cursor-pointer rounded-md overflow-hidden bg-white/5 border border-white/8"
            :class="{ 'ring-2 ring-orange-500/60': i === selectedIndex }"
          >
            <img v-if="isImageFile(item)" :src="item.url" class="w-full h-24 object-cover" />
            <div
              v-else
              class="w-full h-24 flex items-center justify-center text-2xs text-gray-500 bg-white/5"
            >
              {{ item.type || "FILE" }}
            </div>

            <div
              class="absolute inset-0 bg-black/55 opacity-0 group-hover:opacity-100 transition flex flex-col justify-end p-2"
            >
              <span class="text-2xs text-white truncate">{{ item.name }}</span>
              <span class="text-[9px] text-gray-300 truncate">{{ formatSize(item.size) }} KB</span>
            </div>
          </div>
        </div>
      </div>

      <div class="flex-1 min-w-0 flex flex-col gap-4">
        <div
          class="bg-white/5 rounded-xl border border-white/8 min-h-80 flex items-center justify-center overflow-hidden relative"
        >
          <template v-if="selectedMedia && isImageFile(selectedMedia)">
            <img
              :src="selectedMedia.url"
              class="max-h-full max-w-full object-contain"
              alt="Selected media preview"
            />
            <img
              v-if="watermarkSrc"
              :src="watermarkSrc"
              :style="watermarkStyle"
              class="object-contain"
              alt="Watermark preview"
            />
          </template>
          <template v-else>
            <div class="text-center text-gray-500">
              <div class="text-sm">No preview available</div>
              <div class="text-xs mt-1">
                {{ selectedMedia?.name || "Select a file on the left" }}
              </div>
            </div>
          </template>
        </div>

        <div class="grid grid-cols-1 xl:grid-cols-[1fr_260px] gap-4 min-w-0">
          <div class="bg-white/5 rounded-xl border border-white/8 p-4 space-y-3 text-sm min-w-0">
            <div class="flex items-center gap-2">
              <span class="text-gray-400 text-xs">📄</span>
              <span class="text-white truncate">{{ selectedMedia?.name || "—" }}</span>
            </div>
            <div class="flex items-center gap-2">
              <span class="text-gray-400 text-xs">🧩</span>
              <span class="text-gray-300 truncate">{{ selectedMedia?.type || "unknown" }}</span>
            </div>
            <div class="flex items-center gap-2">
              <span class="text-gray-400 text-xs">📦</span>
              <span class="text-gray-300">{{
                selectedMedia ? `${formatSize(selectedMedia.size)} KB` : "—"
              }}</span>
            </div>
            <div class="flex items-center gap-2">
              <span class="text-gray-400 text-xs">📁</span>
              <span class="text-gray-300 truncate">{{ selectedMedia?.path || "—" }}</span>
            </div>
          </div>

          <div class="bg-white/5 rounded-xl border border-white/8 p-4 space-y-4 min-w-0">
            <div class="flex flex-col gap-1.5">
              <span class="text-2xs text-gray-500 uppercase tracking-widest">Watermark image</span>
              <div class="flex gap-1.5">
                <div
                  class="flex-1 min-w-0 bg-white/5 border border-white/8 rounded-md px-2.5 py-1.5 text-xs text-white/80 truncate cursor-pointer hover:bg-white/8 transition"
                  :title="watermarkPath || 'Click to browse'"
                  @click="browseWatermark"
                >
                  {{ watermarkPath || "Click to browse…" }}
                </div>
                <button
                  class="shrink-0 px-2.5 py-1.5 bg-white/8 hover:bg-white/12 border border-white/8 rounded-md text-2xs text-gray-400 hover:text-white transition cursor-pointer"
                  @click="browseWatermark"
                >
                  Browse
                </button>
              </div>
            </div>

            <div class="grid grid-cols-[72px_1fr] gap-3 items-start">
              <div class="flex flex-col gap-1.5 shrink-0">
                <span class="text-2xs text-gray-500 uppercase tracking-widest">Position</span>
                <div
                  class="w-18 h-18 grid grid-cols-3 grid-rows-3 gap-0.5 p-1.5 bg-white/5 border border-white/8 rounded-lg"
                >
                  <button
                    v-for="p in POSITIONS"
                    :key="p.value"
                    :style="{ gridColumn: p.col, gridRow: p.row }"
                    :title="p.value"
                    :class="[
                      'flex items-center justify-center rounded text-[9px] font-bold transition cursor-pointer select-none',
                      position === p.value
                        ? 'bg-orange-500 text-white'
                        : 'text-gray-600 hover:text-gray-300 hover:bg-white/10',
                    ]"
                    @click="position = p.value"
                  >
                    {{ p.label }}
                  </button>
                </div>
              </div>

              <div class="flex flex-col gap-3.5 pt-4 min-w-0">
                <div class="flex flex-col gap-1">
                  <div class="flex items-center justify-between">
                    <span class="text-2xs text-gray-500 uppercase tracking-widest">Size</span>
                    <span class="text-2xs font-mono text-orange-400">{{ wmSize }}%</span>
                  </div>
                  <input
                    v-model.number="wmSize"
                    type="range"
                    min="1"
                    max="100"
                    step="1"
                    class="accent-orange-500 w-full h-1 cursor-pointer"
                  />
                </div>
                <div class="flex flex-col gap-1">
                  <div class="flex items-center justify-between">
                    <span class="text-2xs text-gray-500 uppercase tracking-widest">Opacity</span>
                    <span class="text-2xs font-mono text-orange-400">{{
                      wmOpacity.toFixed(2)
                    }}</span>
                  </div>
                  <input
                    v-model.number="wmOpacity"
                    type="range"
                    min="0.05"
                    max="1"
                    step="0.05"
                    class="accent-orange-500 w-full h-1 cursor-pointer"
                  />
                </div>
              </div>
            </div>

            <div class="flex flex-col gap-1.5">
              <span class="text-2xs text-gray-500 uppercase tracking-widest">Output folder</span>
              <div class="flex gap-1.5">
                <div
                  class="flex-1 min-w-0 bg-white/5 border border-white/8 rounded-md px-2.5 py-1.5 text-xs text-white/80 truncate cursor-pointer hover:bg-white/8 transition"
                  :title="destDir || 'Auto-filled when source is picked'"
                  @click="browseOutput"
                >
                  {{ destDir || "Auto-filled when source is picked" }}
                </div>
                <button
                  class="shrink-0 px-2.5 py-1.5 bg-white/8 hover:bg-white/12 border border-white/8 rounded-md text-2xs text-gray-400 hover:text-white transition cursor-pointer"
                  @click="browseOutput"
                >
                  Browse
                </button>
              </div>
            </div>

            <button
              :disabled="!isValid"
              :class="[
                'mt-2 w-full py-2 rounded-lg text-sm font-medium transition',
                isValid
                  ? 'bg-orange-500 hover:bg-orange-400 text-white cursor-pointer'
                  : 'bg-white/5 text-gray-600 cursor-not-allowed',
              ]"
              @click="run"
            >
              Run watermark <kbd v-if="isValid" class="ml-1.5 text-2xs opacity-60">↩</kbd>
            </button>
          </div>
        </div>
      </div>
    </div>

    <div
      v-else-if="step === 'processing'"
      class="h-full flex flex-col items-center justify-center gap-5"
    >
      <div class="relative w-[72px] h-[72px]">
        <svg class="w-full h-full -rotate-90" viewBox="0 0 72 72">
          <circle
            cx="36"
            cy="36"
            r="30"
            fill="none"
            stroke="rgba(255,255,255,0.08)"
            stroke-width="5"
          />
          <circle
            cx="36"
            cy="36"
            r="30"
            fill="none"
            stroke="#f97316"
            stroke-width="5"
            stroke-linecap="round"
            :stroke-dasharray="`${2 * Math.PI * 30}`"
            :stroke-dashoffset="`${2 * Math.PI * 30 * (1 - pct / 100)}`"
            style="transition: stroke-dashoffset 0.25s ease"
          />
        </svg>
        <span
          class="absolute inset-0 flex items-center justify-center text-xs font-mono font-semibold"
          >{{ pct }}%</span
        >
      </div>

      <div class="text-center">
        <div class="text-sm font-medium text-white">Adding watermarks…</div>
        <div class="text-xs text-gray-400 mt-1">
          {{ progressState.processed }} / {{ progressState.total }} files
        </div>
      </div>

      <div class="w-56 h-1 bg-white/8 rounded-full overflow-hidden">
        <div
          class="h-full bg-orange-500 rounded-full transition-all duration-300 ease-out"
          :style="{ width: `${pct}%` }"
        />
      </div>

      <div class="flex items-center justify-between w-56 text-2xs text-gray-500">
        <span class="truncate max-w-[75%]" :title="progressState.currentFile">{{
          progressState.currentFile || "—"
        }}</span>
        <span class="font-mono shrink-0 ml-2">{{ elapsedLabel }}</span>
      </div>

      <div
        v-if="processing.errors.length"
        class="px-3 py-1 rounded-full bg-red-500/10 border border-red-500/20 text-2xs text-red-400"
      >
        {{ processing.errors.length }} error{{ processing.errors.length > 1 ? "s" : "" }}
      </div>
    </div>

    <div
      v-else-if="step === 'done' && result"
      class="h-full flex flex-col items-center justify-center gap-5"
    >
      <div class="text-4xl">{{ result.errors === 0 ? "✅" : "⚠️" }}</div>

      <div class="text-center">
        <div class="text-sm font-semibold text-white">
          {{ result.errors === 0 ? "Done!" : "Completed with errors" }}
        </div>
        <div class="text-xs text-gray-400 mt-1">
          {{ result.total }} file{{ result.total !== 1 ? "s" : "" }} ·
          {{ formatMs(result.elapsedMs) }}
        </div>
      </div>

      <div class="flex gap-8 text-center">
        <div>
          <div class="text-xl font-semibold text-green-400">{{ result.total - result.errors }}</div>
          <div class="text-2xs text-gray-500 mt-0.5">Succeeded</div>
        </div>
        <div v-if="result.errors > 0">
          <div class="text-xl font-semibold text-red-400">{{ result.errors }}</div>
          <div class="text-2xs text-gray-500 mt-0.5">Failed</div>
        </div>
        <div>
          <div class="text-xl font-semibold text-white">{{ formatMs(result.elapsedMs) }}</div>
          <div class="text-2xs text-gray-500 mt-0.5">Elapsed</div>
        </div>
      </div>

      <button
        class="px-4 py-1.5 bg-white/8 hover:bg-white/12 border border-white/8 rounded-lg text-xs text-gray-300 hover:text-white transition cursor-pointer"
        @click="step = 'configure'"
      >
        ← Run again
      </button>
    </div>
  </ToolLayout>
</template>