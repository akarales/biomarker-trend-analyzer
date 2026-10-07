// Records docs/demo.gif: one clinician flow in ~15 s against the running
// dev app (API :8003 on the committed demo data + `pnpm dev` on :5174).
//
//   cd frontend
//   pnpm exec playwright install ffmpeg      # once (tool, not a dependency)
//   node scripts/record-demo.mjs             # restart the API afterwards to drop the demo review
//
// Flow: triage (worst unreviewed first) → SYN-01 HbA1c card → chart hover
// → explained signals → acknowledge the prRI signal → streamed "explain
// this drift" draft (offline stub through the real streaming pipeline).
import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdtempSync, rmSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { chromium } from '@playwright/test';

const BASE = process.env.DEMO_URL ?? 'http://localhost:5174';
const OUT = new URL('../../docs/demo.gif', import.meta.url).pathname;
const size = { width: 1280, height: 800 };

const dir = mkdtempSync(join(tmpdir(), 'bta-demo-'));
const browser = await chromium.launch({ executablePath: process.env.PW_CHROMIUM_PATH ?? '/usr/bin/google-chrome' });
const context = await browser.newContext({ viewport: size, recordVideo: { dir, size } });
const page = await context.newPage();
const started = Date.now();
await page.goto(BASE);
const triage = page.getByRole('navigation', { name: 'Patients' });
await triage.getByRole('button').first().waitFor();
await page.getByRole('region', { name: 'Biomarkers' }).waitFor();
await page.waitForTimeout(1400);
const lead = (Date.now() - started) / 1000 - 1.2; // trim the blank page load

// open the HbA1c card → chart, then sweep the cursor across the results
await page.getByRole('region', { name: 'Biomarkers' }).getByRole('button', { name: /^HBA1C/ }).click();
const chart = page.getByRole('group', { name: /Hemoglobin A1c trend/ });
await chart.waitFor();
await page.waitForTimeout(500);
await page.getByRole('region', { name: 'Hemoglobin A1c trend' }).evaluate((el) => el.scrollIntoView({ behavior: 'smooth', block: 'start' }));
await page.waitForTimeout(1300);
const box = await chart.boundingBox();
for (let i = 0; i <= 28; i += 1) {
  await page.mouse.move(box.x + 60 + ((box.width - 80) * i) / 28, box.y + box.height * 0.45);
  await page.waitForTimeout(90);
}
await page.waitForTimeout(1400);
await page.mouse.move(box.x + box.width / 2, box.y - 40);

// the signals behind the status, then acknowledge the first one
const signals = page.getByRole('region', { name: 'Signals' });
await signals.evaluate((el) => el.scrollIntoView({ behavior: 'smooth', block: 'start' }));
await page.waitForTimeout(2200);
await signals.getByRole('button', { name: 'Acknowledge' }).first().click();
await signals.getByText(/Acknowledged/).first().waitFor();
await page.waitForTimeout(1800);

// streamed draft for the reviewing clinician
const explain = page.getByRole('region', { name: 'Explain this drift' });
await explain.evaluate((el) => el.scrollIntoView({ behavior: 'smooth', block: 'start' }));
await page.waitForTimeout(1200);
await explain.getByRole('button', { name: /^Explain HBA1C/ }).click();
// the draft grows the page: follow it while it streams
await explain.getByRole('article', { name: 'AI draft' }).waitFor();
await explain.evaluate((el) => el.scrollIntoView({ behavior: 'smooth', block: 'start' }));
await explain.getByRole('button', { name: 'Copy draft' }).waitFor({ timeout: 60_000 });
await page.waitForTimeout(2500);

const video = await page.video().path();
await context.close();
await browser.close();

// webm → GIF (< 3 MB): played 1.15×, 7 fps; denoise and drop near-duplicate
// frames (kept as longer delays), 720 px wide, 48 colours
const filters =
  'setpts=PTS/1.15,fps=7,hqdn3d=3:3:8:8,mpdecimate=hi=64*24:lo=64*8:frac=0.2,scale=720:-1:flags=lanczos,' +
  'split[a][b];[a]palettegen=max_colors=48:stats_mode=diff[p];[b][p]paletteuse=dither=none:diff_mode=rectangle';
execFileSync('ffmpeg', ['-y', '-loglevel', 'error', '-ss', String(Math.max(0, lead)), '-i', video, '-vf', filters, '-fps_mode', 'vfr', '-loop', '0', OUT]);
copyFileSync(video, join(tmpdir(), 'bta-demo-raw.webm')); // for re-encoding without re-recording
rmSync(dir, { recursive: true, force: true });
const seconds = execFileSync('ffprobe', ['-v', 'error', '-show_entries', 'format=duration', '-of', 'csv=p=0', OUT]).toString().trim();
console.log(`${OUT}: ${(statSync(OUT).size / 1e6).toFixed(2)} MB, ${Number(seconds).toFixed(1)} s`);
