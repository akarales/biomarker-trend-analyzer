// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { ErrorBanner } from '@/shared/components/ErrorBanner';
import { useAnalyzer } from '@/state';
import { report, series } from '@/test/fixtures';

import { BiomarkerCard } from './biomarker';
import { TrendChart, TrendPanel } from './chart';
import { PatientList } from './patients';
import { UploadPanel } from './upload';

const initial = useAnalyzer.getState();

afterEach(() => {
  cleanup();
  useAnalyzer.setState(initial, true);
});

describe('BiomarkerCard', () => {
  it('shows z, trend and status, and selects by code', () => {
    const onSelect = vi.fn();
    render(<BiomarkerCard report={report('LDL', { latest_z: 2.345, status: 'watch' })} selected onSelect={onSelect} />);
    const card = screen.getByRole('button', { name: /^LDL/ });
    expect(card.textContent).toContain('z=2.3 · flat');
    expect(card.textContent).toContain('watch');
    expect(card.className).toContain('ring-2');
    fireEvent.click(card);
    expect(onSelect).toHaveBeenCalledWith('LDL');
  });

  it('renders em dashes for missing detector values', () => {
    render(<BiomarkerCard report={report('TSH', { latest_z: null, trend: null })} selected={false} onSelect={() => {}} />);
    expect(screen.getByRole('button').textContent).toContain('z=— · —');
  });
});

describe('TrendChart', () => {
  it('titles the chart and draws one anomaly marker', () => {
    const { container } = render(<TrendChart series={series('alice', 'HBA1C')} />);
    expect(screen.getByRole('heading', { name: 'HBA1C (%) · 3 readings' })).toBeTruthy();
    expect(container.querySelectorAll('circle')).toHaveLength(1);
    expect(screen.getByText('alert')).toBeTruthy();
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
