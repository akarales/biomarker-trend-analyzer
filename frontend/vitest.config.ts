import { defineConfig, mergeConfig } from 'vitest/config';

import viteConfig from './vite.config.ts';

// Workers inherit this. The v1 chart parses naive `taken_at` strings as
// local time but anomaly times are UTC epochs, so dots only line up in UTC
// (known issue, fixed with the M4 chart rewrite).
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
