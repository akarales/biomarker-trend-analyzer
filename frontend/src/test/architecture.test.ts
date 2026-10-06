/// <reference types="node" />
/**
 * Structure rules from AGENTS.md, enforced: colours only from tokens,
 * ~300-line files, features talk to each other only through `index.ts`,
 * and lower layers (api, state, shared) never import features or the app.
 */
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const SRC = fileURLToPath(new URL('..', import.meta.url));
const MAX_LINES = 300;

function walk(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
}

const files = walk(SRC)
  .map((p) => relative(SRC, p).replaceAll('\\', '/'))
  .filter((p) => /\.(ts|tsx)$/.test(p) && !p.startsWith('components/ui/'));
const source = (p: string) => readFileSync(join(SRC, p), 'utf8');
const isTest = (p: string) => /\.test\.tsx?$/.test(p) || p.startsWith('test/');
// both `import x from '…'` and side-effect `import '…'`
const imports = (p: string) => [...source(p).matchAll(/(?:from|import)\s*\(?'(@\/[^']+)'/g)].map((m) => m[1]);

describe('architecture', () => {
  it('keeps every source file under the soft line limit', () => {
    const long = files
      .filter((p) => !isTest(p))
      .map((p) => [p, source(p).split('\n').length] as const)
      .filter(([, n]) => n > MAX_LINES);
    expect(long).toEqual([]);
  });

  it('has no hex colour literals outside shared/domain/tokens.ts', () => {
    const offenders = files
      .filter((p) => !isTest(p) && p !== 'shared/domain/tokens.ts')
      .filter((p) => /#[0-9a-fA-F]{6}\b|#[0-9a-fA-F]{3}\b(?![-\w])/.test(source(p).replace(/\/\/.*|\/\*[\s\S]*?\*\//g, '')));
    expect(offenders).toEqual([]);
  });

  it('features import other features only through their index', () => {
    const offenders = files.flatMap((p) => {
      const own = p.match(/^features\/([^/]+)\//)?.[1];
      return imports(p)
        .filter((spec) => {
          const m = spec.match(/^@\/features\/([^/]+)(\/.+)?$/);
          return m && m[1] !== own && m[2];
        })
        .map((spec) => `${p} -> ${spec}`);
    });
    expect(offenders).toEqual([]);
  });

  it('api, state and shared never depend on features or the app shell', () => {
    const offenders = files
      .filter((p) => /^(api|state|shared)\//.test(p) && !isTest(p))
      .flatMap((p) => imports(p).filter((s) => /^@\/(features|app)\b/.test(s)).map((s) => `${p} -> ${s}`));
    expect(offenders).toEqual([]);
  });

  it('every feature exposes an index.ts', () => {
    const features = [...new Set(files.map((p) => p.match(/^features\/([^/]+)\//)?.[1]).filter(Boolean))];
    expect(features.length).toBeGreaterThan(0);
    expect(features.filter((f) => !files.includes(`features/${f}/index.ts`))).toEqual([]);
  });
});
