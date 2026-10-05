import type { BiomarkerSeries } from '@/api/types';

interface Props {
  series: BiomarkerSeries;
}

const WIDTH = 880;
const HEIGHT = 320;
const PAD = 40;

/**
 * Hand-rolled SVG trend chart: values over time, personal-baseline band
 * (median ± robust σ), and anomaly markers. No chart library — the chart
 * math is simple enough to own.
 */
export function TrendChart({ series }: Props) {
  const points = series.observations.map((o) => ({
    x: new Date(o.taken_at).getTime(),
    y: o.value,
  }));
  // Anomaly timestamps arrive as epoch seconds; chart x is epoch ms.
  const anomalyTimes = new Set(
    series.report.anomalies.map((a) => a.t * 1000),
  );

  const xs = points.map((p) => p.x);
  const ys = points.map((p) => p.y);
  const xMin = Math.min(...xs);
  const xMax = Math.max(...xs, xMin + 1);
  const yMin = Math.min(...ys, series.report.baseline.median - series.report.baseline.robust_std);
  const yMax = Math.max(...ys, series.report.baseline.median + series.report.baseline.robust_std);
  const yPad = (yMax - yMin) * 0.1 || 1;
  const yLo = yMin - yPad;
  const yHi = yMax + yPad;

  const sx = (x: number) => PAD + ((x - xMin) / (xMax - xMin)) * (WIDTH - 2 * PAD);
  const sy = (y: number) => HEIGHT - PAD - ((y - yLo) / (yHi - yLo)) * (HEIGHT - 2 * PAD);

  const path = points
    .map((p, i) => `${i === 0 ? 'M' : 'L'}${sx(p.x).toFixed(1)},${sy(p.y).toFixed(1)}`)
    .join(' ');

  const { median, robust_std: sigma } = series.report.baseline;
  const bandTop = sy(median + sigma);
  const bandBottom = sy(median - sigma);
  const medianLine = sy(median);

  const statusColor: Record<string, string> = {
    alert: '#b91c1c',
    watch: '#b45309',
    normal: '#15803d',
  };

  return (
    <div className="rounded-lg border border-line bg-panel p-3">
      <div className="mb-2 flex items-center justify-between">
        <h3 className="text-sm font-semibold">
          {series.code} ({series.report.unit}) ·{' '}
          {series.observations.length} readings
        </h3>
        <span
          className="rounded px-2 py-0.5 text-xs font-medium text-white"
          style={{ backgroundColor: statusColor[series.report.status] ?? '#0369a1' }}
        >
          {series.report.status}
        </span>
      </div>
      <svg viewBox={`0 0 ${WIDTH} ${HEIGHT}`} className="w-full">
        {/* baseline band: median ± robust σ */}
        <rect
          x={PAD}
          y={bandTop}
          width={WIDTH - 2 * PAD}
          height={Math.max(bandBottom - bandTop, 0)}
          fill="#0f766e"
          opacity={0.08}
        />
        <line
          x1={PAD}
          x2={WIDTH - PAD}
          y1={medianLine}
          y2={medianLine}
          stroke="#0f766e"
          strokeDasharray="6 4"
          strokeWidth={1.5}
        />
        <path d={path} fill="none" stroke="#0369a1" strokeWidth={2} />
        {points.map((p, i) =>
          anomalyTimes.has(p.x) ? (
            <circle key={i} cx={sx(p.x)} cy={sy(p.y)} r={5} fill="#b91c1c" />
          ) : null,
        )}
        <text x={PAD} y={HEIGHT - 10} className="fill-muted" fontSize={10}>
          {series.observations[0]?.taken_at.slice(0, 10)}
        </text>
        <text
          x={WIDTH - PAD}
          y={HEIGHT - 10}
          textAnchor="end"
          className="fill-muted"
          fontSize={10}
        >
          {series.observations[series.observations.length - 1]?.taken_at.slice(0, 10)}
        </text>
        <text x={PAD} y={bandTop - 4} className="fill-muted" fontSize={10}>
          {`baseline ${(median + sigma).toFixed(2)}`}
        </text>
      </svg>
      <p className="mt-1 text-xs text-muted">
        Latest {series.report.latest ?? '—'} · z ={' '}
        {series.report.latest_z?.toFixed(2) ?? '—'} · trend{' '}
        {series.report.trend ?? '—'} · slope{' '}
        {series.report.slope_per_day ? `${series.report.slope_per_day.toFixed(3)}/day` : '—'}
      </p>
    </div>
  );
}
