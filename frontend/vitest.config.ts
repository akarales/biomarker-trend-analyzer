import { defineConfig, mergeConfig } from 'vitest/config';

import viteConfig from './vite.config.ts';

// Workers inherit this. The chart works on epoch seconds (time-zone free);
// UTC keeps date labels in tests identical on every machine.
process.env.TZ ??= 'UTC';

// Unit tests: pure domain logic + the zustand store (network mocked).
// Component tests (*.test.tsx) opt into jsdom with `// @vitest-environment jsdom`.
export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      environment: 'node',
      include: ['src/**/*.test.{ts,tsx}'],
    },
  }),
);
