import { defineConfig } from "vite";
import { resolve } from "node:path";

const rootDir = import.meta.dirname;

export default defineConfig({
  root: resolve(rootDir, "public"),
  resolve: {
    alias: {
      "/pkg": resolve(rootDir, "pkg"),
    },
  },
  server: {
    port: 3000,
    host: true,
  },
  build: {
    outDir: resolve(rootDir, "dist"),
    emptyOutDir: true,
  },
});
