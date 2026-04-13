<script setup lang="ts">
import { ref, onMounted, watch } from "vue";
import { useElementSize } from "@vueuse/core";
import type { MediaItem } from "../types";

const props = defineProps<{
  mediaItems: MediaItem[];
  activeMediaSlug: string;
}>();

const emit = defineEmits<{ update: [slug: string] }>();

const containerEl = ref<HTMLElement>();
const listEl = ref<HTMLUListElement>();
const x = ref(0);
const { width: containerW } = useElementSize(containerEl);

function scrollToActive(slug: string) {
  const index = props.mediaItems.findIndex((item) => item.slug === slug);
  if (index === -1) return;
  const item = listEl.value?.children[index] as HTMLElement | undefined;
  if (!item) return;
  x.value =
    x.value + (containerW.value / 2 - (item.getBoundingClientRect().left + item.offsetWidth / 2));
}

onMounted(() => scrollToActive(props.activeMediaSlug));
watch(
  () => props.activeMediaSlug,
  (slug) => scrollToActive(slug),
);
</script>

<template>
  <div
    ref="containerEl"
    class="relative flex shrink-0 border-t border-white/5 overflow-hidden"
    style="height: 72px"
  >
    <nav class="w-full overflow-hidden">
      <ul ref="listEl" class="flex flex-row items-center gap-1.5 p-1.5 h-full">
        <li
          v-for="media in mediaItems"
          :key="media.slug"
          class="shrink-0"
          :style="`transform: translateX(${x}px) translateZ(0)`"
        >
          <button
            class="block overflow-hidden rounded transition-all duration-300"
            :class="
              media.slug === activeMediaSlug
                ? 'ring-1 ring-primary-500 opacity-100 scale-105'
                : 'opacity-40 hover:opacity-70 hover:scale-105'
            "
            @click="emit('update', media.slug)"
          >
            <img
              :src="media.thumbnailUrl"
              :alt="media.title"
              width="56"
              height="56"
              loading="lazy"
              class="w-14 h-14 object-cover"
            />
          </button>
        </li>
      </ul>
    </nav>
  </div>
</template>