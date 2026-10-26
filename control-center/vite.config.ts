import { readFileSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { resolve } from "node:path";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const repoRoot = resolve(fileURLToPath(new URL(".", import.meta.url)), "..");
const demoFiles = [
  { url: "/codex-theme.json", fileName: "codex-theme.json", source: resolve(repoRoot, "codex-theme.json"), type: "application/json" },
  { url: "/media/poster.jpg", fileName: "media/poster.jpg", source: resolve(repoRoot, "media/poster.jpg"), type: "image/jpeg" },
  { url: "/media/startup.mp4", fileName: "media/startup.mp4", source: resolve(repoRoot, "media/startup.mp4"), type: "video/mp4" }
];

function demoAssets() {
  return {
    name: "codex-demo-assets",
    configureServer(server: { middlewares: { use: (handler: (request: any, response: any, next: () => void) => void) => void } }) {
      server.middlewares.use((request, response, next) => {
        const pathname = request.url?.split("?")[0];
        const asset = demoFiles.find((item) => item.url === pathname);
        if (!asset || !statSync(asset.source, { throwIfNoEntry: false })) {
          next();
          return;
        }
        response.statusCode = 200;
        response.setHeader("Content-Type", asset.type);
        response.end(readFileSync(asset.source));
      });
    },
    generateBundle(this: { emitFile: (asset: { type: "asset"; fileName: string; source: Buffer }) => void }) {
      for (const asset of demoFiles) {
        this.emitFile({ type: "asset", fileName: asset.fileName, source: readFileSync(asset.source) });
      }
    }
  };
}

export default defineConfig({
  base: "./",
  plugins: [react(), demoAssets()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true
  }
});
