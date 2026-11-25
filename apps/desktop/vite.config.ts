import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri usa el puerto 1420 en desarrollo
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // Ignorar el directorio src-tauri para no recargar por cambios en Rust
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    // Tauri usa Chromium/WebView2 — no necesitar compatibilidad con navegadores viejos
    target: process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari13",
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
});
