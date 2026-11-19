import { fileURLToPath } from "node:url";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// Tauri expects a fixed port and must not have Vite clear its output.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: { alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) } },
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: {
    // The Tauri CSP is font-src 'self'; inlined data: fonts would be blocked.
    assetsInlineLimit: (filePath) =>
      /\.woff2?$/.test(filePath) ? false : undefined,
  },
});
