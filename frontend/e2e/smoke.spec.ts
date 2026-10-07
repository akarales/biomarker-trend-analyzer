import { expect, test, type Page } from '@playwright/test';

/**
 * The core flow on the committed demo data (demo/: Synthea subset +
 * hand-made edge cases, see demo/PROVENANCE.md): first patient
 * auto-selected, signals explained, patient switch, CSV + FHIR upload.
 */
const PROMPT = 'Select a biomarker card to see its trend.';

async function openPatient(page: Page, id: string) {
  await page.getByRole('button', { name: new RegExp(`^${id}\\d`) }).click();
  await expect(page.getByText(PROMPT)).toBeVisible();
  await page.waitForLoadState('networkidle');
}

test('browse drift cards and explained signals', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(String(e)));

  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Biomarker Trend Analyzer' })).toBeVisible();
  await expect(page.getByText('not medical advice')).toBeVisible();
  await expect(page.getByRole('button', { name: /^EDGE-01\d/ })).toHaveClass(/font-semibold/);
  await expect(page.getByText(PROMPT)).toBeVisible();

  // EDGE-01: TSH rising on levothyroxine → alert, explained
  const tsh = page.getByRole('button', { name: /^TSH/ });
  await expect(tsh).toContainText('alert · Personal reference interval');
  await tsh.click();
  await expect(page.getByRole('heading', { name: 'TSH (m[IU]/L) · 10 readings' })).toBeVisible();
  const signals = page.getByRole('region', { name: 'Signals' });
  await expect(signals).toContainText("above this patient's personal reference interval");
  await expect(signals).toContainText('subclinical hypothyroidism range');
  await expect(signals).toContainText('clinician review required');

  // switching patient with a card selected clears the chart
  await openPatient(page, 'SYN-01');
  await expect(page.getByRole('heading', { level: 3 })).toHaveCount(0);
  const hba1c = page.getByRole('button', { name: /^HBA1C/ });
  await expect(hba1c).toContainText('alert');
  await hba1c.click();
  await expect(signals).toContainText('diabetes range (ADA ≥ 6.5 %)');

  // EDGE-02: creatinine in mg/dL and µmol/L, normalised; trend detected
  await openPatient(page, 'EDGE-02');
  await page.getByRole('button', { name: /^CREAT/ }).click();
  await expect(page.getByRole('heading', { name: 'CREAT (mg/dL) · 24 readings' })).toBeVisible();
  await expect(signals).toContainText('Creatinine (serum/plasma) is rising');

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
  await expect(page.getByText(/Inserted 0 observations .* \(1 already stored, skipped\)/)).toBeVisible();
  await expect(box).toHaveValue('');

  const bundle = {
    resourceType: 'Bundle',
    type: 'collection',
    entry: [
      {
        resource: {
          resourceType: 'Observation',
          status: 'final',
          code: { coding: [{ system: 'http://loinc.org', code: '3016-3' }] },
          subject: { reference: 'Patient/UPLOAD-1' },
          effectiveDateTime: '2026-09-20T08:00:00Z',
          valueQuantity: { value: 2.1, unit: 'm[IU]/L', system: 'http://unitsofmeasure.org', code: 'm[IU]/L' },
        },
      },
      {
        resource: {
          resourceType: 'Observation',
          status: 'final',
          code: { coding: [{ system: 'http://loinc.org', code: '2345-7' }] },
          subject: { reference: 'Patient/UPLOAD-1' },
          effectiveDateTime: '2026-09-20T08:00:00Z',
          valueQuantity: { value: 99, unit: 'mg/dL', system: 'http://unitsofmeasure.org', code: 'mg/dL' },
        },
      },
    ],
  };
  await box.fill(JSON.stringify(bundle));
  await upload.click();
  await expect(page.getByText(/FHIR: inserted 1 observations .*Not imported: 1× no analyte profile for LOINC 2345-7/)).toBeVisible();
  await expect(page.getByRole('button', { name: /^UPLOAD-1\d/ })).toBeVisible();

  await box.fill('garbage,header');
  await upload.click();
  await expect(page.getByText(/csv error: schema mismatch.*\(request [\w-]+\)/)).toBeVisible();
  await expect(box).toHaveValue('garbage,header');
});
