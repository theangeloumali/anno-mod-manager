import { fileURLToPath } from "node:url";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import { viteSingleFile } from "vite-plugin-singlefile";

// `--mode mockup` emits one self-contained mockup/index.html that opens from disk with a double-click.
export default defineConfig(({ mode }) => {
  const isMockup = mode === "mockup";
  return {
    plugins: [react(), tailwindcss(), ...(isMockup ? [viteSingleFile()] : [])],
    resolve: {
      alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
    },
    // Tauri expects a fixed port and must not have Vite clear its output.
    clearScreen: false,
    server: { port: 5173, strictPort: true },
    build: isMockup
      ? { outDir: "mockup", emptyOutDir: false }
      : {
          // The Tauri CSP is font-src 'self'; inlined data: fonts would be blocked.
          assetsInlineLimit: (filePath: string) =>
            /\.woff2?$/.test(filePath) ? false : undefined,
        },
  };
});
