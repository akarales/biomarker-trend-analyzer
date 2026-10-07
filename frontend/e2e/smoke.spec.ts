import { expect, test, type Page } from '@playwright/test';

/**
 * The clinician flow on the committed demo data (demo/PROVENANCE.md):
 * triage worst-first, explained signals, as-of analysis, shareable URL,
 * CSV + FHIR upload.
 */
const PROMPT = 'Select a biomarker card to see its trend.';
const patient = (page: Page, id: string) =>
  page.getByRole('navigation', { name: 'Patients' }).getByRole('button', { name: new RegExp(`^${id}(?!\\d)`) });
const card = (page: Page, code: string) =>
  page.getByRole('region', { name: 'Biomarkers' }).getByRole('button', { name: new RegExp(`^${code}`) });

async function openPatient(page: Page, id: string) {
  await patient(page, id).click();
  await expect(patient(page, id)).toHaveAttribute('aria-current', 'true');
  await expect(page.getByText(PROMPT)).toBeVisible();
  await page.waitForLoadState('networkidle');
}

test('triage, explained signals, as-of and a shareable view', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(String(e)));

  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Biomarker Trend Analyzer' })).toBeVisible();
  await expect(page.getByText('not medical advice')).toBeVisible();
  // worst first: SYN-01 (3 alerting biomarkers) is opened automatically
  await expect(patient(page, 'SYN-01')).toHaveAttribute('aria-current', 'true');
  await expect(patient(page, 'SYN-01')).toContainText('3 alert');
  await expect(patient(page, 'EDGE-03')).toContainText('No rule fired');

  const hba1c = card(page, 'HBA1C');
  await expect(hba1c).toContainText('Alert');
  await expect(hba1c).toContainText('Why: Personal reference interval');
  await expect(hba1c).toContainText('Population4.00–5.60 %');
  await hba1c.click();
  const trend = page.getByRole('region', { name: 'Hemoglobin A1c trend' });
  await expect(trend.getByRole('heading', { name: 'HBA1C (%) · 13 readings' })).toBeVisible();
  await expect(page.getByRole('region', { name: 'Signals' })).toContainText('diabetes range (ADA ≥ 6.5 %)');
  await expect(page.getByRole('list', { name: 'Chart legend' })).toContainText('Personal range 5.94–6.43 %');
  await expect(page).toHaveURL(/\?patient=SYN-01&code=HBA1C$/);

  // EDGE-01: TSH rising — then as of 2026-03-31 (before the rise) no rule fires
  await openPatient(page, 'EDGE-01');
  await card(page, 'TSH').click();
  await expect(page.getByRole('region', { name: 'Signals' })).toContainText('subclinical hypothyroidism range');
  const asOf = page.getByLabel('As of');
  await asOf.fill('2026-03-31');
  await asOf.press('Enter');
  await expect(page).toHaveURL(/as_of=2026-03-31/);
  await expect(card(page, 'TSH')).toContainText('No rule fired');
  await expect(page.getByRole('region', { name: 'Thyrotropin (TSH) trend' })).toContainText('TSH (mIU/L) · 8 readings');
  await page.getByRole('button', { name: 'Latest' }).click();
  await expect(card(page, 'TSH')).toContainText('Alert');

  // a shared link restores patient, biomarker, as-of and window
  await page.goto('/?patient=EDGE-02&code=CREAT&window=365');
  await expect(page.getByRole('heading', { name: 'CREAT (mg/dL) · 24 readings' })).toBeVisible();
  await expect(page.getByRole('combobox', { name: 'Trend window' })).toHaveValue('365');
  await expect(page.getByRole('region', { name: 'Signals' })).toContainText('Creatinine (serum/plasma) is rising');

  expect(errors).toEqual([]);
});

test('upload CSV and FHIR, with duplicates and validation errors', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByText(PROMPT)).toBeVisible();
  const box = page.getByRole('textbox', { name: 'Lab results (CSV or FHIR JSON)' });
  const upload = page.getByRole('button', { name: 'Upload', exact: true });
  await expect(upload).toBeDisabled();

  // a CSV row that duplicates a demo FHIR result (same patient, analyte, time)
  await box.fill('patient_id,code,value,unit,taken_at,source\nEDGE-04,4548-4,6.0,%,2026-09-01T08:30:00,lab');
  await upload.click();
  await expect(page.getByRole('status')).toContainText(/Inserted 0 observations .* \(1 already stored, skipped\)/);
  await expect(box).toHaveValue('');

  const observation = (code: string, value: number, unit: string) => ({
    resource: {
      resourceType: 'Observation',
      status: 'final',
      code: { coding: [{ system: 'http://loinc.org', code }] },
      subject: { reference: 'Patient/UPLOAD-1' },
      effectiveDateTime: '2026-09-20T08:00:00Z',
      valueQuantity: { value, unit, system: 'http://unitsofmeasure.org', code: unit },
    },
  });
  const bundle = { resourceType: 'Bundle', type: 'collection', entry: [observation('3016-3', 2.1, 'm[IU]/L'), observation('2345-7', 99, 'mg/dL')] };
  await box.fill(JSON.stringify(bundle));
  await upload.click();
  await expect(page.getByRole('status')).toContainText(/FHIR: inserted 1 observations .*Not imported: 1× no analyte profile for LOINC 2345-7/);
  await expect(patient(page, 'UPLOAD-1')).toBeVisible();

  await box.fill('garbage,header');
  await upload.click();
  await expect(page.getByRole('status')).toContainText(/csv error: schema mismatch.*\(request [\w-]+\)/);
  await expect(box).toHaveValue('garbage,header');
});
