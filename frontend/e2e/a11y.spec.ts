import { AxeBuilder } from '@axe-core/playwright';
import { expect, test, type Page } from '@playwright/test';

/**
 * WCAG 2.2 A/AA (axe) in every state of the workspace — landing, a patient
 * with the chart, the table alternative, an open select, an upload result,
 * an as-of view and the mobile layout. No serious or critical violation
 * may remain. Plus: the chart is usable from the keyboard alone.
 */
async function seriousViolations(page: Page): Promise<string[]> {
  const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa']).analyze();
  return results.violations
    .filter((v) => v.impact === 'serious' || v.impact === 'critical')
    .map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(' ')).slice(0, 3).join(' | ')}`);
}

async function ready(page: Page, url = '/') {
  await page.goto(url);
  await expect(page.getByRole('navigation', { name: 'Patients' }).getByRole('button').first()).toBeVisible();
  await page.waitForLoadState('networkidle');
}

test('landing: triage list and biomarker cards', async ({ page }) => {
  await ready(page);
  await expect(page.getByRole('region', { name: 'Biomarkers' })).toBeVisible();
  expect(await seriousViolations(page)).toEqual([]);
});

test('chart, legend and signals; then the table view', async ({ page }) => {
  await ready(page, '/?patient=SYN-01&code=HBA1C');
  await expect(page.getByRole('region', { name: 'Hemoglobin A1c trend' })).toBeVisible();
  expect(await seriousViolations(page)).toEqual([]);
  await page.getByRole('tab', { name: 'Table' }).click();
  await expect(page.getByRole('table')).toBeVisible();
  expect(await seriousViolations(page)).toEqual([]);
});

test('window change and an as-of view', async ({ page }) => {
  await ready(page, '/?patient=EDGE-02&code=CREAT');
  await page.getByRole('combobox', { name: 'Trend window' }).selectOption({ label: '1 year' });
  await expect(page).toHaveURL(/window=365/);
  await page.waitForLoadState('networkidle');
  expect(await seriousViolations(page)).toEqual([]);
  await ready(page, '/?patient=EDGE-01&code=TSH&as_of=2026-03-31');
  await expect(page.getByRole('region', { name: 'Thyrotropin (TSH) trend' })).toBeVisible();
  await expect(page.getByLabel('As of')).toHaveValue('2026-03-31');
  expect(await seriousViolations(page)).toEqual([]);
});

test('upload outcome and validation error', async ({ page }) => {
  await ready(page);
  const box = page.getByRole('textbox', { name: 'Lab results (CSV or FHIR JSON)' });
  await box.fill('garbage,header');
  await page.getByRole('button', { name: 'Upload', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('schema mismatch');
  expect(await seriousViolations(page)).toEqual([]);
});

test('the chart works from the keyboard alone', async ({ page }) => {
  await ready(page, '/?patient=EDGE-03&code=CREAT');
  const chart = page.getByRole('group', { name: /^Creatinine \(serum\/plasma\) trend, 12 results/ });
  await chart.focus();
  const live = page.locator('[aria-live="polite"]');
  await expect(live).toContainText('Result 12 of 12');
  await page.keyboard.press('ArrowLeft');
  await page.keyboard.press('ArrowLeft');
  await expect(live).toContainText('Result 10 of 12, 2026-09-02: 1.94 mg/dL — outside personal range; change beyond RCV');
  await page.keyboard.press('Home');
  await expect(live).toContainText('Result 1 of 12');
});

test.describe('mobile', () => {
  test.use({ viewport: { width: 390, height: 844 } });
  test('stacked layout with chart', async ({ page }) => {
    await ready(page, '/?patient=EDGE-02&code=CREAT');
    await expect(page.getByRole('region', { name: 'Creatinine (serum/plasma) trend' })).toBeVisible();
    expect(await seriousViolations(page)).toEqual([]);
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(overflow).toBeLessThanOrEqual(0);
  });
});
