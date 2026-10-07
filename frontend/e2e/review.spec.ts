import { AxeBuilder } from '@axe-core/playwright';
import { expect, test } from '@playwright/test';

/**
 * Review workflow against the real API: acknowledge → triage and card
 * update, the audit trail grows, reopen needs a reason, identifiers are
 * refused, and the review form passes axe. Uses EDGE-04 (one watch signal),
 * which no other spec depends on.
 */
test('acknowledge, reopen and the audit trail', async ({ page }) => {
  await page.goto('/?patient=EDGE-04&code=HBA1C');
  const triage = page.getByRole('navigation', { name: 'Patients' }).getByRole('button', { name: /^EDGE-04(?!\d)/ });
  await expect(triage).toContainText('1 unreviewed');
  const signals = page.getByRole('region', { name: 'Signals' });
  await expect(signals).toContainText('Unreviewed');

  await signals.getByRole('button', { name: 'Acknowledge' }).click();
  await expect(signals).toContainText(/Acknowledged\s*by demo-clinician/);
  await expect(triage).toContainText('all reviewed');
  await expect(triage).toContainText('Watch'); // the finding stays visible
  await expect(page.getByRole('region', { name: 'Biomarkers' }).getByRole('button', { name: /^HBA1C/ })).toContainText('All signals reviewed');
  const history = page.getByRole('region', { name: 'Review history' });
  await expect(history.getByRole('listitem')).toHaveCount(1);
  await expect(history).toContainText('Acknowledged · Clinical threshold');
  await expect(history).toContainText('At decision: status watch');

  // reopen needs a reason; identifiers are refused by the server
  await signals.getByRole('button', { name: 'Reopen…' }).click();
  const reason = signals.getByLabel(/Why reopen it\?/);
  await reason.fill('see MRN 12345678');
  await signals.getByRole('button', { name: 'Reopen signal' }).click();
  await expect(signals.getByRole('alert')).toContainText('looks like it contains an ID number');
  const axe = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa']).analyze();
  expect(axe.violations.filter((v) => v.impact === 'serious' || v.impact === 'critical').map((v) => v.id)).toEqual([]);

  await reason.fill('fasting glucose since then is higher');
  await signals.getByRole('button', { name: 'Reopen signal' }).click();
  await expect(signals).toContainText('Unreviewed');
  await expect(triage).toContainText('1 unreviewed');
  await expect(history.getByRole('listitem')).toHaveCount(2);
  await expect(history.getByRole('listitem').first()).toContainText('Reopened');
  await expect(history.getByRole('listitem').first()).toContainText('“fasting glucose since then is higher”');
});
