import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// Tauri serves the frontend from a fixed port in development and from
// `dist/` in a release build. The port is pinned because `tauri.conf.json`
// names it too, and a mismatch shows up as a blank window rather than as an
// error.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // The Rust side has its own rebuild loop; watching it from Vite would
      // mean two toolchains reacting to one save.
      ignored: ["**/src-tauri/**"],
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test-setup.ts"],
    // The Rust core has its own suite; this one covers the interface only.
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
