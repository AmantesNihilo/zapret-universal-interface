import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte({ hot: false })],
  resolve: {
    alias: { $lib: fileURLToPath(new URL("./src/lib", import.meta.url)) },
    conditions: ["browser"]
  },
  test: {
    environment: "jsdom",
    include: ["tests/**/*.test.ts"],
    maxWorkers: 1,
    clearMocks: true
  }
});
