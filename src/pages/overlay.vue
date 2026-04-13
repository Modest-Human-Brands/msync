<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from "vue";
import { useIntervalFn } from "@vueuse/core";
import { open } from "@tauri-apps/plugin-dialog";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useRouter } from "vue-router";
import ToolLayout from "../layouts/ToolLayout.vue";
import OverlaySidePanel from "../components/Overlay/OverlaySidePanel.vue";
import MediaFlimstrip from "../components/MediaFlimstrip.vue";
import { isMediaFile } from "../utils";
import type { MediaItem } from "../types";

type RustMediaItem = MediaItem & { path: string };

type OverlayProgressEvent = {
  current: number;
  total: number;
  path: string;
  status: "done" | "error";
  error?: string | null;
};

type BulkOverlayReport = {
  succeeded: string[];
  failed: [string, string][];
};

type ProcessStatus = "idle" | "processing" | "done";
type Stage = "awaiting-source" | "awaiting-watermark" | "ready" | "processing" | "done";

const router = useRouter();

const mediaItems = ref<RustMediaItem[]>([]);
const srcDir = ref("");
const currentIndex = ref(0);
const currentItem = computed(() => mediaItems.value[currentIndex.value]);

const watermarkPath = ref("");
const watermarkSrc = computed(() =>
  watermarkPath.value ? convertFileSrc(watermarkPath.value) : "",
);
const watermarkName = computed(() => watermarkPath.value.split(/[\\/]/).at(-1) ?? "");
const watermarkNaturalSize = ref({ w: 0, h: 0 });

const canvasWidth = ref(1920);
const canvasHeight = ref(1080);

const hAlign = ref<"left" | "center" | "right">("right");
const vAlign = ref<"top" | "middle" | "bottom">("bottom");

const wmScale = ref(20);
const heightLocked = ref(true);
const wmManualHeight = ref(0);
const wmOpacity = ref(0.8);
const wmPadding = ref(0);

const wmAspectRatio = computed(() =>
  watermarkNaturalSize.value.h > 0
    ? watermarkNaturalSize.value.w / watermarkNaturalSize.value.h
    : 1,
);
const wmWidthPx = computed(() => Math.round((canvasWidth.value * wmScale.value) / 100));
const wmHeightPx = computed(() =>
  heightLocked.value
    ? Math.round(wmWidthPx.value / wmAspectRatio.value)
    : wmManualHeight.value || Math.round(wmWidthPx.value / wmAspectRatio.value),
);

const padding = computed(() => Math.round(Math.min(canvasWidth.value, canvasHeight.value) * 0.02));
const positionXPx = computed(() => {
  if (hAlign.value === "left") return padding.value;
  if (hAlign.value === "center") return Math.round((canvasWidth.value - wmWidthPx.value) / 2);
  return canvasWidth.value - wmWidthPx.value - padding.value;
});
const positionYPx = computed(() => {
  if (vAlign.value === "top") return padding.value;
  if (vAlign.value === "middle") return Math.round((canvasHeight.value - wmHeightPx.value) / 2);
  return canvasHeight.value - wmHeightPx.value - padding.value;
});

const mediaAspectRatio = computed(() => {
  const ar = currentItem.value?.metadata?.aspectRatio;
  if (!ar) return "16/9";
  const [w, h] = ar.split(":").map(Number);
  return w && h ? `${w}/${h}` : "16/9";
});

const watermarkPreviewStyle = computed(() => {
  const pad = `${wmPadding.value}%`;
  const s: Record<string, string> = {
    position: "absolute",
    width: `${wmScale.value}%`,
    opacity: String(wmOpacity.value),
    pointerEvents: "none",
  };
  s.left = hAlign.value === "left" ? pad : hAlign.value === "center" ? "50%" : "auto";
  s.right = hAlign.value === "right" ? pad : "auto";
  s.top = vAlign.value === "top" ? pad : vAlign.value === "middle" ? "50%" : "auto";
  s.bottom = vAlign.value === "bottom" ? pad : "auto";
  s.transform =
    hAlign.value === "center" && vAlign.value === "middle"
      ? "translate(-50%, -50%)"
      : hAlign.value === "center"
        ? "translateX(-50%)"
        : vAlign.value === "middle"
          ? "translateY(-50%)"
          : "none";
  return s;
});

const processStatus = ref<ProcessStatus>("idle");
const processing = ref({
  total: 0,
  processed: 0,
  currentFile: "",
  startedAt: 0,
  errors: [] as string[],
});
const result = ref<{ total: number; errors: number; elapsedMs: number } | null>(null);
const now = ref(Date.now());

const { pause, resume } = useIntervalFn(
  () => {
    now.value = Date.now();
  },
  200,
  { immediate: false },
);

const pct = computed(() => {
  const { total, processed } = processing.value;
  return total > 0 ? Math.round((processed / total) * 100) : 0;
});
const elapsedLabel = computed(() => {
  if (!processing.value.startedAt) return "0s";
  const s = (now.value - processing.value.startedAt) / 1000;
  return s < 60 ? `${s.toFixed(1)}s` : `${Math.floor(s / 60)}m ${Math.floor(s % 60)}s`;
});

const stage = computed<Stage>(() => {
  if (processStatus.value === "processing") return "processing";
  if (processStatus.value === "done") return "done";
  if (!mediaItems.value.length) return "awaiting-source";
  if (!watermarkPath.value) return "awaiting-watermark";
  return "ready";
});

const statusLabel = computed(() => {
  if (stage.value === "done" && result.value)
    return `${result.value.total - result.value.errors}/${result.value.total} succeeded · ${formatMs(result.value.elapsedMs)}`;
});

let progressUnlisten: UnlistenFn | null = null;
let completeUnlisten: UnlistenFn | null = null;

async function startListening() {
  await stopListening();
  progressUnlisten = await listen<OverlayProgressEvent>("overlay-progress", ({ payload }) => {
    requestAnimationFrame(() => {
      processing.value = {
        ...processing.value,
        total: payload.total,
        processed: payload.current,
        currentFile: payload.path,
        errors:
          payload.status === "error" && payload.error
            ? [...processing.value.errors, `${payload.path}: ${payload.error}`]
            : processing.value.errors,
      };
    });
  });
  completeUnlisten = await listen<BulkOverlayReport>("overlay-complete", ({ payload }) => {
    result.value = {
      total: payload.succeeded.length + payload.failed.length,
      errors: payload.failed.length,
      elapsedMs: Date.now() - processing.value.startedAt,
    };
  });
}

async function stopListening() {
  progressUnlisten?.();
  completeUnlisten?.();
  progressUnlisten = null;
  completeUnlisten = null;
}

watch(
  () => processing.value.currentFile,
  (filePath) => {
    if (!filePath) return;
    const idx = mediaItems.value.findIndex((m) => m.path === filePath);
    if (idx !== -1) currentIndex.value = idx;
  },
);

async function loadMedia(path: string) {
  srcDir.value = path;
  const files = await invoke<RustMediaItem[]>("list_files", { dir: path });
  mediaItems.value = files
    .map((f) => ({ ...f, thumbnailUrl: convertFileSrc(f.path) }))
    .filter((f) => isMediaFile(f.path));
  currentIndex.value = 0;
}

async function browseSrc() {
  const selected = await open({ multiple: false, directory: true });
  if (!selected) return;
  await loadMedia(Array.isArray(selected) ? selected[0]! : selected);
}

async function browseWatermark() {
  const selected = await open({
    multiple: false,
    filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "webp", "svg", "avif"] }],
  });
  if (!selected) return;
  watermarkPath.value = Array.isArray(selected) ? selected[0]! : selected;
  watermarkNaturalSize.value = { w: 0, h: 0 };
}

function onWatermarkLoaded(e: Event) {
  const img = e.target as HTMLImageElement;
  watermarkNaturalSize.value = { w: img.naturalWidth, h: img.naturalHeight };
}

async function run() {
  if (stage.value !== "ready") return;

  processStatus.value = "processing";
  processing.value = { total: 0, processed: 0, currentFile: "", startedAt: Date.now(), errors: [] };
  result.value = null;
  now.value = Date.now();

  resume();
  await startListening();

  const startedAt = Date.now();
  const hMap = { left: 0, center: 0.5, right: 1 } as const;
  const vMap = { top: 0, middle: 0.5, bottom: 1 } as const;

  try {
    const report = await invoke<BulkOverlayReport>("add_overlays", {
      mediaPaths: mediaItems.value.map((f) => f.path),
      overlayPath: watermarkPath.value,
      outputDir: `${srcDir.value}_watermarked`,
      overlayOptions: {
        scale_ratio: wmScale.value / 100,
        position: [hMap[hAlign.value], vMap[vAlign.value]] as [number, number],
        padding_ratio: wmPadding.value / 100,
        opacity: wmOpacity.value,
      },
      threadLimit: null,
    });
    result.value = {
      total: report.succeeded.length + report.failed.length,
      errors: report.failed.length,
      elapsedMs: Date.now() - startedAt,
    };
  } catch {
    result.value = {
      total: processing.value.processed,
      errors: 1,
      elapsedMs: Date.now() - startedAt,
    };
  } finally {
    await stopListening();
    pause();
    processStatus.value = "done";
  }
}

function formatMs(ms: number) {
  const s = ms / 1000;
  return s < 60 ? `${s.toFixed(1)}s` : `${Math.floor(s / 60)}m ${(s % 60).toFixed(0)}s`;
}

function handleAction(key: "enter" | "escape") {
  if (key !== "enter") return;
  if (stage.value === "awaiting-source") browseSrc();
  else if (stage.value === "awaiting-watermark") browseWatermark();
  else if (stage.value === "ready") run();
  else if (stage.value === "done") processStatus.value = "idle";
}

onUnmounted(() => {
  stopListening();
  pause();
});
</script>

<template>
  <ToolLayout :prevent-escape="processStatus === 'processing'" @action="handleAction">
    <template #header>
      <div class="flex items-center gap-2 min-w-0">
        <button
          class="text-white/40 hover:text-white transition-colors shrink-0"
          @click="router.back()"
        >
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <polyline points="15 18 9 12 15 6" />
          </svg>
        </button>
        <span class="text-white/30 shrink-0">/</span>
        <span class="text-white/50 shrink-0 uppercase text-xs tracking-wider">Add Overlay</span>
        <span
          v-if="mediaItems.length"
          class="ml-1 px-2 py-0.5 rounded bg-primary-500/20 text-primary-500 text-xs font-medium shrink-0"
        >
          {{ mediaItems.length }} assets
        </span>
      </div>
    </template>

    <img v-if="watermarkSrc" :src="watermarkSrc" class="hidden" @load="onWatermarkLoaded" />

    <section class="flex h-full overflow-hidden">
      <div class="flex-1 flex flex-col min-w-0 overflow-hidden bg-black">
        <div class="flex-1 relative overflow-hidden flex items-center justify-center">
          <div
            v-if="currentItem?.thumbnailUrl"
            class="relative overflow-hidden"
            :style="{ aspectRatio: mediaAspectRatio, maxWidth: '100%', maxHeight: '100%' }"
          >
            <video
              v-if="currentItem.type === 'video'"
              :src="currentItem.thumbnailUrl"
              class="w-full h-full object-contain select-none"
              controls
              autoplay
              loop
              playsinline
            ></video>

            <img
              v-else
              :src="currentItem.thumbnailUrl"
              :alt="currentItem.title"
              class="w-full h-full object-contain select-none"
              draggable="false"
            />
            <img
              v-if="watermarkSrc"
              :src="watermarkSrc"
              :style="watermarkPreviewStyle"
              class="select-none"
              draggable="false"
            />
          </div>

          <div v-else class="flex flex-col items-center justify-center gap-3 text-white/20">
            <svg
              width="40"
              height="40"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <rect x="3" y="3" width="18" height="18" rx="2" />
              <circle cx="8.5" cy="8.5" r="1.5" />
              <polyline points="21 15 16 10 5 21" />
            </svg>
            <span class="text-sm">No assets loaded</span>
            <button
              class="px-4 py-2 rounded bg-white/10 hover:bg-white/20 text-white text-xs transition-colors"
              @click="browseSrc"
            >
              Browse folder
            </button>
          </div>

          <div
            v-if="currentItem"
            class="absolute top-3 left-3 flex items-center gap-2 pointer-events-none"
          >
            <span
              v-if="currentItem.metadata?.resolution"
              class="bg-success-600/20 text-success-600 text-2xs font-mono px-2 py-0.5 rounded border border-success-600/20"
            >
              {{ currentItem.metadata.resolution }}
            </span>
            <span class="text-xs text-white/50">
              Asset {{ currentIndex + 1 }} of {{ mediaItems.length }} ({{ currentItem.title }})
            </span>
          </div>

          <button
            v-if="currentIndex > 0"
            class="absolute left-3 top-1/2 -translate-y-1/2 w-8 h-8 flex items-center justify-center rounded-full bg-black/50 hover:bg-black/80 text-white/70 hover:text-white transition-colors"
            @click="currentIndex--"
          >
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <polyline points="15 18 9 12 15 6" />
            </svg>
          </button>
          <button
            v-if="currentIndex < mediaItems.length - 1"
            class="absolute right-3 top-1/2 -translate-y-1/2 w-8 h-8 flex items-center justify-center rounded-full bg-black/50 hover:bg-black/80 text-white/70 hover:text-white transition-colors"
            @click="currentIndex++"
          >
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <polyline points="9 18 15 12 9 6" />
            </svg>
          </button>
        </div>

        <MediaFlimstrip
          v-if="mediaItems.length"
          :media-items="mediaItems as MediaItem[]"
          :active-media-slug="currentItem?.slug ?? ''"
          @update="
            (slug) => {
              currentIndex = mediaItems.findIndex((m) => m.slug === slug);
            }
          "
        />
      </div>

      <OverlaySidePanel
        v-model:h-align="hAlign"
        v-model:v-align="vAlign"
        v-model:wm-scale="wmScale"
        v-model:wm-opacity="wmOpacity"
        v-model:height-locked="heightLocked"
        v-model:wm-manual-height="wmManualHeight"
        :canvas-width="canvasWidth"
        :watermark-name="watermarkName"
        :watermark-src="watermarkSrc"
        :watermark-natural-size="watermarkNaturalSize"
        :position-x="positionXPx"
        :position-y="positionYPx"
        :wm-width-px="wmWidthPx"
        :wm-height-px="wmHeightPx"
        @browse-watermark="browseWatermark"
      />
    </section>

    <template #footer>
      <template v-if="stage === 'processing'">
        <div class="flex flex-col gap-1.5 w-full">
          <div class="flex items-center justify-between">
            <span class="text-xs text-white/40 font-mono"
              >{{ processing.processed }}&thinsp;/&thinsp;{{ processing.total }} files</span
            >
            <span class="text-xs text-white/40 font-mono">{{ pct }}% · {{ elapsedLabel }}</span>
          </div>
          <div class="w-full bg-white/10 rounded-full overflow-hidden" style="height: 3px">
            <div
              class="h-full bg-primary-500 rounded-full transition-all duration-300 ease-out"
              :style="{ width: `${pct}%` }"
            />
          </div>
        </div>
      </template>

      <template v-else>
        <div class="flex items-center gap-1.5">
          <span class="text-xs text-white/40 uppercase tracking-wider">{{ statusLabel }}</span>
        </div>

        <div class="flex items-center gap-1.5">
          <button
            class="px-2.5 py-1 rounded bg-white/10 hover:bg-white/20 text-white/70 hover:text-white text-xs font-medium tracking-wider transition-colors"
            @click="router.back()"
          >
            ESC
          </button>
          <button
            class="px-2.5 py-1 rounded bg-white/10 hover:bg-white/20 text-white/70 hover:text-white text-xs font-medium tracking-wider transition-colors"
            @click="router.back()"
          >
            BACK
          </button>

          <button
            v-if="stage === 'awaiting-source'"
            class="px-3 py-1 rounded bg-white/10 hover:bg-white/20 text-white text-xs font-medium tracking-wider transition-colors"
            @click="browseSrc"
          >
            LOAD FOLDER →
          </button>
          <button
            v-else-if="stage === 'awaiting-watermark'"
            class="px-3 py-1 rounded bg-white/10 hover:bg-white/20 text-white text-xs font-medium tracking-wider transition-colors"
            @click="browseWatermark"
          >
            LOAD WATERMARK →
          </button>
          <button
            v-else-if="stage === 'ready'"
            class="px-3 py-1 rounded bg-primary-500 hover:bg-primary-400 text-white text-xs font-semibold tracking-wider transition-colors flex items-center gap-1.5"
            @click="run"
          >
            <svg width="10" height="10" viewBox="0 0 24 24" fill="currentColor">
              <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
            </svg>
            ENTER RUN
          </button>
          <button
            v-else-if="stage === 'done'"
            class="px-3 py-1 rounded bg-white/10 hover:bg-white/20 text-white text-xs font-medium tracking-wider transition-colors"
            @click="processStatus = 'idle'"
          >
            RUN AGAIN
          </button>
        </div>
      </template>
    </template>
  </ToolLayout>
</template>