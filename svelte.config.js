import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  // Umožní písať <script lang="ts"> v .svelte súboroch.
  preprocess: vitePreprocess(),
};
