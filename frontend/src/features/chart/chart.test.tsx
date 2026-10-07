// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';

import { useAnalyzer } from '@/state';
import { report, series } from '@/test/fixtures';

import { ChartLegend } from './ChartLegend';
import { SignalList } from './SignalList';
import { TrendChart } from './TrendChart';
import { TrendPanel } from './TrendPanel';

const initial = useAnalyzer.getState();

afterEach(() => {
  cleanup();
  useAnalyzer.setState(initial, true);
});

describe('TrendChart', () => {
  it('is one focusable region read result by result with the keyboard', () => {
    render(<TrendChart report={series('alice', 'HBA1C').report} label="Hemoglobin A1c trend" />);
    const chart = screen.getByRole('group', { name: /^Hemoglobin A1c trend, 3 results in %/ });
    expect(chart.getAttribute('tabindex')).toBe('0');
    const live = () => document.querySelector('[aria-live="polite"]')?.textContent ?? '';

    fireEvent.focus(chart);
    expect(live()).toBe('Result 3 of 3, 2026-03-01: 7.00 % — outside personal range');
    fireEvent.keyDown(chart, { key: 'ArrowLeft' });
    expect(live()).toMatch(/^Result 2 of 3, 2026-02-01: 5.60 %$/);
    fireEvent.keyDown(chart, { key: 'Home' });
    expect(live()).toMatch(/^Result 1 of 3/);
    fireEvent.keyDown(chart, { key: 'ArrowLeft' });
    expect(live()).toMatch(/^Result 1 of 3/);
    fireEvent.keyDown(chart, { key: 'End' });
    expect(live()).toMatch(/^Result 3 of 3/);
    fireEvent.keyDown(chart, { key: 'Escape' });
    expect(live()).toBe('');
  });

  it('marks results outside the personal range and draws the band', () => {
    const { container } = render(<TrendChart report={series('alice', 'HBA1C').report} label="trend" />);
    expect(container.querySelectorAll('svg circle')).toHaveLength(3);
    expect(container.querySelector('svg')?.getAttribute('aria-hidden')).toBe('true');
  });
});

describe('TrendPanel', () => {
  it('prompts until a series is loaded', () => {
    useAnalyzer.setState({ reports: [report('HBA1C')], series: null });
    render(<TrendPanel />);
    expect(screen.getByText('Select a biomarker card to see its trend.')).toBeTruthy();
  });

  it('switches between the chart and its table alternative', async () => {
    useAnalyzer.setState({ series: series('alice', 'HBA1C'), reports: [series('alice', 'HBA1C').report] });
    render(<TrendPanel />);
    expect(screen.getByRole('region', { name: 'Hemoglobin A1c trend' })).toBeTruthy();
    expect(screen.getByRole('heading', { name: 'HBA1C (%) · 3 readings' })).toBeTruthy();
    await userEvent.click(screen.getByRole('tab', { name: 'Table' }));
    const table = screen.getByRole('table');
    expect(table.textContent).toContain('Hemoglobin A1c results: 3 results, newest first, values normalised to %');
    const rows = screen.getAllByRole('row');
    expect(rows[1].textContent).toContain('2026-03-01');
    expect(rows[1].textContent).toContain('7.00');
    expect(rows[1].textContent).toContain('7 %');
    expect(rows[1].textContent).toContain('outside personal range');
  });
});

describe('derived series', () => {
  it('says it is derived, from what and how, and labels the input column', async () => {
    const derived = {
      from: 'CREAT',
      method: 'derived from serum creatinine with the race-free 2021 CKD-EPI creatinine equation',
      gender: 'male',
      birth_year: 1957,
    };
    const s = { ...series('EDGE-02', 'EGFR'), derived };
    useAnalyzer.setState({ series: s, reports: [s.report] });
    render(<TrendPanel />);
    expect(screen.getByText(/Derived series — not a measured result\. Derived from serum creatinine/)).toBeTruthy();
    expect(document.body.textContent).toContain('Inputs: CREAT results, male, born 1957.');
    await userEvent.click(screen.getByRole('tab', { name: 'Table' }));
    expect(screen.getByRole('columnheader', { name: 'CREAT (input)' })).toBeTruthy();
  });
});

describe('ChartLegend', () => {
  it('names every drawn layer with its values', () => {
    const r = series('alice', 'HBA1C').report;
    render(
      <ChartLegend
        report={{ ...r, change_point: { t: r.points[2].t, detected_t: r.points[2].t, before: 5.5, after: 7, change: 0.27 } }}
      />,
    );
    const legend = screen.getByRole('list', { name: 'Chart legend' }).textContent ?? '';
    expect(legend).toContain('Personal range 5.38–5.82 % (from 3 results 2026-01-01 → 2026-02-01)');
    expect(legend).toContain('Population range 4.00–5.60 %');
    expect(legend).toContain('Shift starting 2026-03-01 (CUSUM)');
  });
});

describe('SignalList', () => {
  it('explains each signal with its source and lists what was not assessed', () => {
    const r = { ...series('alice', 'HBA1C').report, not_assessed: [{ rule: 'trend' as const, reason: 'needs ≥ 4 results' }] };
    render(<SignalList report={r} />);
    const region = screen.getByRole('region', { name: 'Signals' }).textContent ?? '';
    expect(region).toContain('AlertPersonal reference interval');
    expect(region).toContain('Source: Coşkun A et al., Clin Chem 2021');
    expect(region).toContain('Trend (Mann–Kendall): needs ≥ 4 results');
    expect(region).toContain('clinician review required');
  });

  it('says that no signal is not a clean bill of health', () => {
    render(<SignalList report={report('LDL')} />);
    expect(screen.getByText(/not a statement that the result is healthy/)).toBeTruthy();
  });
});
