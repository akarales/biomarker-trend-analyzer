// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { ErrorBanner } from '@/shared/components/ErrorBanner';
import { StatusChip } from '@/shared/components/StatusChip';
import { useAnalyzer } from '@/state';
import { entry, report, series } from '@/test/fixtures';

import { BiomarkerCard, BiomarkerCards } from './biomarker';
import { PatientList } from './patients';
import { UploadPanel } from './upload';

const initial = useAnalyzer.getState();

afterEach(() => {
  cleanup();
  useAnalyzer.setState(initial, true);
});

describe('StatusChip', () => {
  it('always carries a word next to the colour', () => {
    render(<StatusChip status="normal" />);
    expect(screen.getByText('No rule fired')).toBeTruthy();
    cleanup();
    render(<StatusChip severity="info" />);
    expect(screen.getByText('Info')).toBeTruthy();
  });
});

describe('BiomarkerCard', () => {
  it('shows latest, personal vs population range, trend and the deciding rule', () => {
    const onSelect = vi.fn();
    render(<BiomarkerCard report={series('alice', 'HBA1C').report} selected onSelect={onSelect} />);
    const card = screen.getByRole('button', { name: /^HBA1C/ });
    expect(card.getAttribute('aria-pressed')).toBe('true');
    const text = card.textContent ?? '';
    expect(text).toContain('7.00 %');
    expect(text).toContain('Personal5.38–5.82 %');
    expect(text).toContain('Population4.00–5.60 %');
    expect(text).toContain('Trendtoo few results');
    expect(text).toContain('Why: Personal reference interval');
    fireEvent.click(card);
    expect(onSelect).toHaveBeenCalledWith('HBA1C');
  });

  it('says what is missing instead of inventing values', () => {
    render(
      <BiomarkerCard
        report={report('LDL', { latest: null, population: null, unit: 'mg/dL' })}
        selected={false}
        onSelect={() => {}}
      />,
    );
    const text = screen.getByRole('button').textContent ?? '';
    expect(text).toContain('not yet (needs 3 earlier results)');
    expect(text).toContain('none in profile');
    expect(text).toContain('No rule fired');
  });
});

describe('BiomarkerCards', () => {
  it('shows demographics in the heading and marks derived series', () => {
    useAnalyzer.setState({
      selectedPatient: 'alice',
      patients: [entry('alice', 'alert')],
      reports: [report('CREAT', { unit: 'mg/dL' }), report('EGFR', { unit: 'mL/min/{1.73_m2}' })],
      summaryDerived: { EGFR: { from: 'CREAT', method: 'm', gender: 'female', birth_year: 1963 } },
      demographics: { patient_id: 'alice', gender: 'female', birth_year: 1963 },
    });
    render(<BiomarkerCards />);
    expect(screen.getByRole('heading', { level: 2 }).textContent).toBe('alice · female, born 1963 · 2 biomarkers');
    expect(screen.getByRole('button', { name: /^EGFR/ }).textContent).toContain('derived from CREAT');
    expect(screen.getByRole('button', { name: /^CREAT/ }).textContent).not.toContain('derived from');
  });
});

describe('PatientList (triage)', () => {
  it('shows status, counts and the top signal; marks and selects', () => {
    const selectPatient = vi.fn(async () => {});
    useAnalyzer.setState({ patients: [entry('alice', 'alert'), entry('bob')], selectedPatient: 'alice', selectPatient });
    render(<PatientList />);
    const nav = screen.getByRole('navigation', { name: 'Patients' });
    const alice = screen.getByRole('button', { name: /^alice/ });
    expect(alice.getAttribute('aria-current')).toBe('true');
    expect(alice.textContent).toContain('Alert');
    expect(alice.textContent).toContain('1 alert · 2 biomarkers · 6 results');
    expect(alice.textContent).toContain('Hemoglobin A1c — Personal reference interval');
    expect(nav.textContent).toContain('no rule fired');
    fireEvent.click(screen.getByRole('button', { name: /^bob/ }));
    expect(selectPatient).toHaveBeenCalledWith('bob');
  });

  it('shows the empty state', () => {
    render(<PatientList />);
    expect(screen.getByText('No data yet — upload a CSV or FHIR bundle.')).toBeTruthy();
  });
});

describe('UploadPanel', () => {
  it('enables Upload only with text and reports the outcome as a status', () => {
    const upload = vi.fn(async () => {});
    useAnalyzer.setState({ upload, uploadMessage: 'Inserted 1 observations' });
    render(<UploadPanel />);
    const button = screen.getByRole('button', { name: 'Upload' }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    fireEvent.change(screen.getByRole('textbox', { name: 'Lab results (CSV or FHIR JSON)' }), { target: { value: 'a,b' } });
    expect(button.disabled).toBe(false);
    fireEvent.click(button);
    expect(upload).toHaveBeenCalled();
    expect(screen.getByRole('status').textContent).toBe('Inserted 1 observations');
  });

  it('loads a chosen FHIR file into the box', async () => {
    render(<UploadPanel />);
    const input = screen.getByLabelText('Load file…') as HTMLInputElement;
    const file = new File(['{"resourceType":"Bundle"}'], 'labs.json', { type: 'application/fhir+json' });
    fireEvent.change(input, { target: { files: [file] } });
    await waitFor(() => expect(useAnalyzer.getState().uploadText).toBe('{"resourceType":"Bundle"}'));
  });
});

describe('ErrorBanner', () => {
  it('renders an alert only with a message', () => {
    const { container, rerender } = render(<ErrorBanner message={null} />);
    expect(container.textContent).toBe('');
    rerender(<ErrorBanner message="Error: internal error" />);
    expect(screen.getByRole('alert').textContent).toBe('Error: internal error');
  });
});
