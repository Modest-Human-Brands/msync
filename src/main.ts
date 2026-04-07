import { createApp } from "vue";
import App from "./App.vue";
import { createRouter, createWebHashHistory } from "vue-router";

import Home from "./pages/index.vue";
import Sync from "./pages/sync.vue";
import Overlay from "./pages/overlay.vue";
// import Reorganize from './pages/reorganize.vue'

const routes = [
  { path: "/", name: "home", component: Home },
  { path: "/sync", name: "sync", component: Sync },
  { path: "/overlay", name: "overlay", component: Overlay },
  // { path: '/reorganize', name: 'reorganize', component: Reorganize },
];

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

createApp(App).use(router).mount("#app");