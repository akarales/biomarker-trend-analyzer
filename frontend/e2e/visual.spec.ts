import { expect, test, type Page } from '@playwright/test';

/**
 * Screenshot comparison used to prove refactors change no pixels
 * (`PW_VISUAL=1 pnpm e2e visual --update-snapshots` on the old code, then
 * `PW_VISUAL=1 pnpm e2e visual` on the new code). Baselines depend on the
 * machine's fonts and the day the drift is computed, so they stay local
 * (gitignored) and the spec is skipped by default.
 */
test.skip(!process.env.PW_VISUAL, 'set PW_VISUAL=1 to compare screenshots');

const shot = { fullPage: true, animations: 'disabled', maxDiffPixels: 0 } as const;
const PROMPT = 'Select a biomarker card to see its trend.';

async function openPatient(page: Page, id: string) {
  await page.getByRole('button', { name: new RegExp(`^${id}\\d`) }).click();
  await expect(page.getByText(PROMPT)).toBeVisible();
  await page.waitForLoadState('networkidle');
}

async function openBiomarker(page: Page, code: string) {
  await page.getByRole('button', { name: new RegExp(`^${code}`) }).click();
  await expect(page.getByRole('heading', { name: new RegExp(`^${code} \\(`) })).toBeVisible();
  await page.waitForLoadState('networkidle');
}

async function upload(page: Page, csv: string, expected: RegExp) {
  await page.getByRole('textbox').fill(csv);
  await page.getByRole('button', { name: 'Upload' }).click();
  await expect(page.getByText(expected)).toBeVisible();
  await page.waitForLoadState('networkidle');
}

async function fresh(page: Page) {
  await page.goto('/');
  await expect(page.getByText(PROMPT)).toBeVisible();
  await page.waitForLoadState('networkidle');
}

test('desktop states', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await fresh(page);
  await expect(page).toHaveScreenshot('01-initial.png', shot);

  await openBiomarker(page, 'HBA1C');
  await expect(page).toHaveScreenshot('02-alice-hba1c.png', shot);

  // patient switches start from a fresh load: v1 raced when a card was
  // selected (it fetched the old code for the new patient) — fixed in M1
  await fresh(page);
  await openPatient(page, 'bob');
  await expect(page).toHaveScreenshot('03-bob.png', shot);
  await openBiomarker(page, 'LDL');
  await expect(page).toHaveScreenshot('04-bob-ldl.png', shot);

  await fresh(page);
  await openPatient(page, 'carol');
  await openBiomarker(page, 'TSH');
  await expect(page).toHaveScreenshot('05-carol-tsh.png', shot);

  await page.getByRole('textbox').fill('alice,HBA1C,5.7');
  await expect(page).toHaveScreenshot('06-upload-typed.png', shot);
  await upload(
    page,
    'patient_id,code,value,unit,taken_at,source\nalice,HBA1C,5.59,%,2026-01-06,synthetic-demo',
    /already stored, skipped/,
  );
  await expect(page).toHaveScreenshot('07-upload-duplicate.png', shot);
  // the message carries a per-request id: mask it, compare everything else
  await upload(page, 'garbage,header', /schema mismatch/);
  await expect(page).toHaveScreenshot('08-upload-error.png', {
    ...shot,
    mask: [page.getByText(/schema mismatch/)],
  });
});

test('narrow viewport', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/');
  await expect(page.getByText(PROMPT)).toBeVisible();
  await openBiomarker(page, 'CREAT');
  await expect(page).toHaveScreenshot('09-narrow-creat.png', shot);
});
