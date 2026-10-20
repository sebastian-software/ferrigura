/**
 * Vitest configuration of the DDLT part of the swimlane corpus. The root configuration excludes
 * `crates/**`, so the capture spec runs with this one, and with the same plugins as the root
 * configuration (the jison grammars and the TypeScript build), in jsdom as the DDLT specs are.
 *
 *   pnpm exec vitest run --config crates/ferrigura-swimlane/tools/vitest.config.ts
 */
import typescript from '@rollup/plugin-typescript';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';
import jison from '../../../.vite/jisonPlugin.js';
import jsonSchemaPlugin from '../../../.vite/jsonSchemaPlugin.js';

export default defineConfig({
  root: fileURLToPath(new URL('../../../', import.meta.url)),
  resolve: {
    extensions: ['.js'],
  },
  plugins: [jison(), jsonSchemaPlugin(), typescript({ compilerOptions: { declaration: false } })],
  test: {
    environment: 'jsdom',
    globals: true,
    include: ['crates/ferrigura-swimlane/tools/*.spec.ts'],
    testTimeout: 1_800_000,
  },
});
