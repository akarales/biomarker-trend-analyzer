import { expect, test, type Page } from '@playwright/test';

/**
 * Screenshot comparison used to prove refactors change no pixels
 * (`PW_VISUAL=1 pnpm e2e visual --update-snapshots` on the old code, then
 * `PW_VISUAL=1 pnpm e2e visual` on the new code). Baselines depend on the
 * machine's fonts, so they stay local (gitignored) and the spec is skipped
 * by default. States use the committed demo data via deep links.
 */
test.skip(!process.env.PW_VISUAL, 'set PW_VISUAL=1 to compare screenshots');

const shot = { fullPage: true, animations: 'disabled', maxDiffPixels: 0 } as const;

async function open(page: Page, url: string) {
  await page.goto(url);
  await expect(page.getByRole('navigation', { name: 'Patients' }).getByRole('button').first()).toBeVisible();
  await page.waitForLoadState('networkidle');
  // let ResizeObserver settle the chart width
  await page.waitForTimeout(150);
}

test('desktop states', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await open(page, '/');
  await expect(page).toHaveScreenshot('01-triage.png', shot);
  await open(page, '/?patient=SYN-01&code=HBA1C');
  await expect(page).toHaveScreenshot('02-syn01-hba1c.png', shot);
  await page.getByRole('tab', { name: 'Table' }).click();
  await expect(page).toHaveScreenshot('03-syn01-hba1c-table.png', shot);
  await open(page, '/?patient=EDGE-02&code=CREAT');
  await expect(page).toHaveScreenshot('04-edge02-creat.png', shot);
  await open(page, '/?patient=EDGE-01&code=TSH&as_of=2026-03-31');
  await expect(page).toHaveScreenshot('05-edge01-tsh-as-of.png', shot);
  await open(page, '/?patient=SYN-04&code=CREAT');
  await expect(page).toHaveScreenshot('06-syn04-dense.png', shot);
});

test('keyboard readout', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await open(page, '/?patient=EDGE-03&code=CREAT');
  await page.getByRole('group', { name: /trend, 12 results/ }).focus();
  await page.keyboard.press('ArrowLeft');
  await page.keyboard.press('ArrowLeft');
  await expect(page).toHaveScreenshot('07-keyboard-readout.png', shot);
});

test('narrow viewport', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await open(page, '/?patient=EDGE-02&code=CREAT');
  await expect(page).toHaveScreenshot('08-narrow.png', shot);
});
