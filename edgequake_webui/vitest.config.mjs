import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  test: {
    globals: true,
    projects: [
      {
        name: "unit-node",
        test: {
          environment: "node",
          include: ["src/**/*.test.ts"],
          exclude: ["src/**/*.test.tsx", "src/**/*.dom.test.ts"],
        },
        resolve: {
          alias: { "@": path.resolve(__dirname, "./src") },
        },
      },
      {
        name: "unit-jsdom",
        test: {
          environment: "jsdom",
          include: ["src/**/*.test.tsx", "src/**/*.dom.test.ts"],
          setupFiles: ["./vitest.setup.ts"],
        },
        resolve: {
          alias: { "@": path.resolve(__dirname, "./src") },
        },
      },
    ],
  },
});
