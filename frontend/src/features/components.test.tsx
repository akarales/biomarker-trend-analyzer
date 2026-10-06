// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { ErrorBanner } from '@/shared/components/ErrorBanner';
import { useAnalyzer } from '@/state';
import { report, series } from '@/test/fixtures';

import { BiomarkerCard } from './biomarker';
import { SignalList, TrendChart, TrendPanel } from './chart';
import { PatientList } from './patients';
import { UploadPanel } from './upload';

const initial = useAnalyzer.getState();

afterEach(() => {
  cleanup();
  useAnalyzer.setState(initial, true);
});

describe('BiomarkerCard', () => {
  it('shows latest, trend, status and the deciding rule, and selects by code', () => {
    const onSelect = vi.fn();
    const r = series('alice', 'HBA1C').report;
    render(<BiomarkerCard report={r} selected onSelect={onSelect} />);
    const card = screen.getByRole('button', { name: /^HBA1C/ });
    expect(card.textContent).toContain('latest 7.00 · —');
    expect(card.textContent).toContain('alert · Personal reference interval');
    expect(card.className).toContain('ring-2');
    fireEvent.click(card);
    expect(onSelect).toHaveBeenCalledWith('HBA1C');
  });

  it('renders em dashes for missing values and no rule when nothing fired', () => {
    render(<BiomarkerCard report={report('TSH', { latest: null })} selected={false} onSelect={() => {}} />);
    const text = screen.getByRole('button').textContent ?? '';
    expect(text).toContain('latest — · —');
    expect(text.endsWith('normal')).toBe(true);
  });
});

describe('TrendChart', () => {
  it('titles the chart, labels the prRI band and marks the result outside it', () => {
    const { container } = render(<TrendChart series={series('alice', 'HBA1C')} />);
    expect(screen.getByRole('heading', { name: 'HBA1C (%) · 3 readings' })).toBeTruthy();
    expect(container.querySelectorAll('circle')).toHaveLength(1);
    expect(screen.getByText('personal range 5.38–5.82')).toBeTruthy();
    expect(screen.getByText('2026-01-01')).toBeTruthy();
    expect(screen.getByText('alert')).toBeTruthy();
  });
});

describe('SignalList', () => {
  it('explains each signal with its source and lists what was not assessed', () => {
    const r = { ...series('alice', 'HBA1C').report, not_assessed: [{ rule: 'trend' as const, reason: 'needs ≥ 4 results' }] };
    render(<SignalList report={r} />);
    const region = screen.getByRole('region', { name: 'Signals' });
    expect(region.textContent).toContain('Personal reference interval · alert');
    expect(region.textContent).toContain('Source: Coşkun A et al., Clin Chem 2021');
    expect(region.textContent).toContain('Trend (Mann–Kendall): needs ≥ 4 results');
    expect(region.textContent).toContain('clinician review required');
  });

  it('says that no signal is not a clean bill of health', () => {
    render(<SignalList report={report('LDL')} />);
    expect(screen.getByText(/not a statement that the result is healthy/)).toBeTruthy();
  });
});

describe('TrendPanel', () => {
  it('prompts until a series is loaded', () => {
    useAnalyzer.setState({ reports: [report('HBA1C')], series: null });
    render(<TrendPanel />);
    expect(screen.getByText('Select a biomarker card to see its trend.')).toBeTruthy();
  });
});

describe('PatientList', () => {
  it('lists patients, bolds the selected one and selects on click', () => {
    const selectPatient = vi.fn(async () => {});
    useAnalyzer.setState({
      patients: [
        { patient_id: 'alice', biomarkers: 4, observations: 192 },
        { patient_id: 'bob', biomarkers: 4, observations: 192 },
      ],
      selectedPatient: 'alice',
      selectPatient,
    });
    render(<PatientList />);
    expect(screen.getByRole('button', { name: /^alice/ }).className).toContain('font-semibold');
    fireEvent.click(screen.getByRole('button', { name: /^bob/ }));
    expect(selectPatient).toHaveBeenCalledWith('bob');
  });

  it('shows the empty state', () => {
    render(<PatientList />);
    expect(screen.getByText('No data yet — upload a CSV.')).toBeTruthy();
  });
});

describe('UploadPanel', () => {
  it('enables Upload only with text and shows the outcome', () => {
    const upload = vi.fn(async () => {});
    useAnalyzer.setState({ upload });
    render(<UploadPanel />);
    const button = screen.getByRole('button', { name: 'Upload' }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    fireEvent.change(screen.getByRole('textbox'), { target: { value: 'a,b' } });
    expect(button.disabled).toBe(false);
    fireEvent.click(button);
    expect(upload).toHaveBeenCalled();
  });
});

describe('ErrorBanner', () => {
  it('renders only with a message', () => {
    const { container, rerender } = render(<ErrorBanner message={null} />);
    expect(container.textContent).toBe('');
    rerender(<ErrorBanner message="Error: internal error" />);
    expect(screen.getByText('Error: internal error')).toBeTruthy();
  });
});
