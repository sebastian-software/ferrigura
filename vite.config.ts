import { defaultExclude, defineConfig } from 'vitest/config';

/** The tests of the harness: `vitest run tools/port` (CI's test job). */
export default defineConfig({
  test: {
    environment: 'node',
    exclude: [
      ...defaultExclude,
      // Worktrees the Claude desktop app creates inside the repository.
      '.claude/**',
      // The Rust port: crates/ferrigura-node/test runs under `node --test`, not vitest.
      'crates/**',
      'target/**',
    ],
  },
});
