// SPDX-License-Identifier: GPL-3.0-or-later
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// Tauri loads the dev server in development and `dist/` in release builds.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "es2022",
    // Loaded from disk by a local app, so one larger bundle is fine.
    chunkSizeWarningLimit: 1200,
  },
  test: {
    environment: "node",
  },
});
