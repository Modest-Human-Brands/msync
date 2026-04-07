import { defineConfig } from "vite-plus";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
import Icons from "unplugin-icons/vite";
import Components from "unplugin-vue-components/vite";
import IconsResolver from "unplugin-icons/resolver";
import { FileSystemIconLoader } from "unplugin-icons/loaders";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  fmt: {},
  lint: {
    ignorePatterns: ["dist/**", "node_modules/**", "temp/**"],
    options: { typeAware: true, typeCheck: true },
  },
  plugins: [vue(), tailwindcss(), Icons({
    customCollections: {
      app: FileSystemIconLoader("./src/assets/icons", (svg) =>
        svg.replace(/^<svg /, '<svg fill="currentColor" ')
      ),
    },
  }),

  Components({
    resolvers: [
      IconsResolver({
        prefix: "Icon", // 👈 this gives <IconOverlay />
        customCollections: ["app"],
      }),
    ],
  }),],
  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`

  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 3000,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
        protocol: "ws",
        host,
        port: 3000,
      }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
});