import { expect, test } from '@playwright/test';

/**
 * The core flow on the seeded synthetic demo (3 patients × 4 biomarkers):
 * first patient auto-selected, open a trend, switch patient, upload.
 */
const PROMPT = 'Select a biomarker card to see its trend.';

test('browse drift cards and trends', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(String(e)));

  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Biomarker Trend Analyzer' })).toBeVisible();
  await expect(page.getByText('not medical advice')).toBeVisible();
  const alice = page.getByRole('button', { name: /^alice\d/ });
  await expect(alice).toHaveClass(/font-semibold/);
  await expect(page.getByText(PROMPT)).toBeVisible();
  for (const code of ['CREAT', 'HBA1C', 'LDL', 'TSH']) {
    await expect(page.getByRole('button', { name: new RegExp(`^${code}`) })).toBeVisible();
  }

  // alice's HbA1c step 5.6 → 7.0 %: alert, explained by the prRI + ADA threshold
  const hba1c = page.getByRole('button', { name: /^HBA1C/ });
  await expect(hba1c).toContainText('alert · Personal reference interval');
  await hba1c.click();
  await expect(page.getByRole('heading', { name: 'HBA1C (%) · 48 readings' })).toBeVisible();
  const signals = page.getByRole('region', { name: 'Signals' });
  await expect(signals).toContainText("above this patient's personal reference interval 5.38–5.82 %");
  await expect(signals).toContainText('diabetes range (ADA ≥ 6.5 %)');
  await expect(signals).toContainText('Sustained shift: since 2026-09-15');
  await expect(signals).toContainText('clinician review required');

  // switching patient with a card selected clears the chart (v1 showed the
  // new patient's series for the old card without a selection)
  await page.getByRole('button', { name: /^bob\d/ }).click();
  await expect(page.getByText(PROMPT)).toBeVisible();
  await page.waitForLoadState('networkidle');
  await expect(page.getByRole('heading', { level: 3 })).toHaveCount(0);

  expect(errors).toEqual([]);
});

test('upload reports duplicates and validation errors', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByText(PROMPT)).toBeVisible();
  const box = page.getByRole('textbox');
  const upload = page.getByRole('button', { name: 'Upload' });
  await expect(upload).toBeDisabled();

  await box.fill('patient_id,code,value,unit,taken_at,source\nalice,HBA1C,5.59,%,2026-01-06,synthetic-demo');
  await upload.click();
  await expect(page.getByText(/Inserted 0 observations .* \(1 already stored, skipped\)/)).toBeVisible();
  await expect(box).toHaveValue('');

  await box.fill('garbage,header');
  await upload.click();
  await expect(page.getByText(/csv error: schema mismatch.*\(request [\w-]+\)/)).toBeVisible();
  await expect(box).toHaveValue('garbage,header');
});
