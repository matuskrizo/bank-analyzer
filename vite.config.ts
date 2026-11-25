import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Tauri očakáva vývojový server na pevnom porte (tauri.conf.json → devUrl).
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false, // nech Vite neprepisuje chyby z Rustu v termináli
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // Zmeny v Ruste sleduje Tauri samo – Vite ich nemusí.
      ignored: ["**/src-tauri/**"],
    },
  },
});
