import type { BiomarkerSeries } from '@/api/schemas';
import { CHART_COLOR, STATUS_COLOR, STATUS_COLOR_FALLBACK, formatDate, formatPct, formatValue } from '@/shared/domain';

import { chartGeometry, HEIGHT, PAD, WIDTH } from './geometry';

interface Props {
  series: BiomarkerSeries;
}

/**
 * Hand-rolled SVG trend chart: analysed values over time, the personal
 * reference interval (prRI) with its set point, and results outside the
 * prRI marked. Axes, thresholds and hover arrive with the M4 chart.
 */
export function TrendChart({ series }: Props) {
  const { report } = series;
  const { points, path, band } = chartGeometry(report);
  const { baseline, trend, unit } = report;
  const first = report.points[0];
  const last = report.latest;

  return (
    <div className="rounded-lg border border-line bg-panel p-3">
      <div className="mb-2 flex items-center justify-between">
        <h3 className="text-sm font-semibold">
          {series.code} ({unit}) · {report.points.length} readings
        </h3>
        <span
          className="rounded px-2 py-0.5 text-xs font-medium text-white"
          style={{ backgroundColor: STATUS_COLOR[report.status] ?? STATUS_COLOR_FALLBACK }}
        >
          {report.status}
        </span>
      </div>
      <svg viewBox={`0 0 ${WIDTH} ${HEIGHT}`} className="w-full" role="img" aria-label={`${series.code} trend`}>
        {band && baseline && (
          <>
            {/* personal reference interval (95 % prediction) */}
            <rect
              x={PAD}
              y={band.top}
              width={WIDTH - 2 * PAD}
              height={Math.max(band.bottom - band.top, 0)}
              fill={CHART_COLOR.baseline}
              opacity={0.08}
            />
            <line
              x1={PAD}
              x2={WIDTH - PAD}
              y1={band.middle}
              y2={band.middle}
              stroke={CHART_COLOR.baseline}
              strokeDasharray="6 4"
              strokeWidth={1.5}
            />
            <text x={PAD} y={band.top - 4} className="fill-muted" fontSize={10}>
              {`personal range ${formatValue(baseline.prri_low)}–${formatValue(baseline.prri_high)}`}
            </text>
          </>
        )}
        <path d={path} fill="none" stroke={CHART_COLOR.series} strokeWidth={2} />
        {points.map((p, i) =>
          p.flagged ? <circle key={i} cx={p.cx} cy={p.cy} r={5} fill={CHART_COLOR.anomaly} /> : null,
        )}
        {first && (
          <text x={PAD} y={HEIGHT - 10} className="fill-muted" fontSize={10}>
            {formatDate(first.t)}
          </text>
        )}
        {last && (
          <text x={WIDTH - PAD} y={HEIGHT - 10} textAnchor="end" className="fill-muted" fontSize={10}>
            {formatDate(last.t)}
          </text>
        )}
      </svg>
      <p className="mt-1 text-xs text-muted">
        Latest {last ? formatValue(last.v) : '—'} · trend {trend?.direction ?? '—'}
        {trend && trend.direction !== 'flat' ? ` (${formatPct(trend.change_per_year)}/yr)` : ''}
        {baseline ? ` · set point ${formatValue(baseline.set_point)} from ${baseline.n} results` : ''}
      </p>
    </div>
  );
}
