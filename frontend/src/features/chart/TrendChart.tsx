import type { BiomarkerSeries } from '@/api/schemas';
import { CHART_COLOR, STATUS_COLOR, STATUS_COLOR_FALLBACK } from '@/shared/domain';

import { chartGeometry, HEIGHT, PAD, WIDTH } from './geometry';

interface Props {
  series: BiomarkerSeries;
}

/**
 * Hand-rolled SVG trend chart: values over time, personal-baseline band
 * (median ± robust σ), and anomaly markers. No chart library — the chart
 * math is simple enough to own (see geometry.ts).
 */
export function TrendChart({ series }: Props) {
  const { points, path, bandTop, bandBottom, medianLine } = chartGeometry(series);
  const { median, robust_std: sigma } = series.report.baseline;

  return (
    <div className="rounded-lg border border-line bg-panel p-3">
      <div className="mb-2 flex items-center justify-between">
        <h3 className="text-sm font-semibold">
          {series.code} ({series.report.unit}) ·{' '}
          {series.observations.length} readings
        </h3>
        <span
          className="rounded px-2 py-0.5 text-xs font-medium text-white"
          style={{ backgroundColor: STATUS_COLOR[series.report.status] ?? STATUS_COLOR_FALLBACK }}
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
          fill={CHART_COLOR.baseline}
          opacity={0.08}
        />
        <line
          x1={PAD}
          x2={WIDTH - PAD}
          y1={medianLine}
          y2={medianLine}
          stroke={CHART_COLOR.baseline}
          strokeDasharray="6 4"
          strokeWidth={1.5}
        />
        <path d={path} fill="none" stroke={CHART_COLOR.series} strokeWidth={2} />
        {points.map((p, i) =>
          p.anomaly ? <circle key={i} cx={p.cx} cy={p.cy} r={5} fill={CHART_COLOR.anomaly} /> : null,
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
