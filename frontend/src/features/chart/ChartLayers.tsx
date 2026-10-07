import { CHART_COLOR, SEVERITY_COLOR, formatValue } from '@/shared/domain';

import { MARGIN, type ChartLayout } from './geometry';

interface Props {
  layout: ChartLayout;
  active: number | null;
}

/** The static SVG drawing (decorative to assistive tech — the wrapper, readout and table carry the meaning). */
export function ChartLayers({ layout, active }: Props) {
  const { width, height, points, prri, population, thresholds, jumps, changePoint, xTicks, yTicks } = layout;
  const left = MARGIN.left;
  const right = width - MARGIN.right;
  const bottom = height - MARGIN.bottom;
  const cursor = active !== null ? points[active] : null;

  return (
    <svg width={width} height={height} aria-hidden="true" className="block select-none">
      {yTicks.map((tick) => (
        <g key={`y${tick.v}`}>
          <line x1={left} x2={right} y1={tick.y} y2={tick.y} stroke={CHART_COLOR.grid} strokeWidth={1} />
          <text x={left - 8} y={tick.y + 4} textAnchor="end" fontSize={11} fill={CHART_COLOR.axis}>
            {formatValue(tick.v)}
          </text>
        </g>
      ))}
      {xTicks.map((tick) => (
        <g key={`x${tick.t}`}>
          <line x1={tick.x} x2={tick.x} y1={bottom} y2={bottom + 4} stroke={CHART_COLOR.axis} />
          <text x={tick.x} y={bottom + 18} textAnchor="middle" fontSize={11} fill={CHART_COLOR.axis}>
            {tick.label}
          </text>
        </g>
      ))}
      {population && (
        <rect
          x={left}
          y={population.top}
          width={right - left}
          height={Math.max(population.bottom - population.top, 0)}
          fill={CHART_COLOR.population}
          opacity={0.14}
        />
      )}
      {prri && (
        <>
          <rect
            x={left}
            y={prri.top}
            width={right - left}
            height={Math.max(prri.bottom - prri.top, 0)}
            fill={CHART_COLOR.prri}
            opacity={0.16}
          />
          <line x1={left} x2={right} y1={prri.middle} y2={prri.middle} stroke={CHART_COLOR.prri} strokeDasharray="6 4" />
        </>
      )}
      {thresholds.map((t) => (
        <g key={`t${t.value}`}>
          <line x1={left} x2={right} y1={t.y} y2={t.y} stroke={SEVERITY_COLOR[t.severity]} strokeDasharray="2 3" />
          <text x={right - 4} y={t.y - 4} textAnchor="end" fontSize={11} fill={SEVERITY_COLOR[t.severity]}>
            {formatValue(t.value)}
          </text>
        </g>
      ))}
      {changePoint && (
        <g>
          <line x1={changePoint.x} x2={changePoint.x} y1={MARGIN.top} y2={bottom} stroke={CHART_COLOR.changePoint} strokeDasharray="4 3" />
          <text x={changePoint.x + 4} y={MARGIN.top + 10} fontSize={11} fill={CHART_COLOR.changePoint}>
            shift
          </text>
        </g>
      )}
      <path d={layout.path} fill="none" stroke={CHART_COLOR.series} strokeWidth={2} />
      {jumps.map((j, i) => (
        <line key={`j${i}`} x1={j.x1} y1={j.y1} x2={j.x2} y2={j.y2} stroke={CHART_COLOR.rcv} strokeWidth={4} strokeLinecap="round" opacity={0.85} />
      ))}
      {points.length <= 120 &&
        points.map((p) => (
          <circle key={p.t} cx={p.cx} cy={p.cy} r={p.outside ? 4.5 : 3} fill={p.outside ? CHART_COLOR.flagged : CHART_COLOR.series} />
        ))}
      {points.length > 120 &&
        points.filter((p) => p.outside).map((p) => <circle key={p.t} cx={p.cx} cy={p.cy} r={3.5} fill={CHART_COLOR.flagged} />)}
      {cursor && (
        <g>
          <line x1={cursor.cx} x2={cursor.cx} y1={MARGIN.top} y2={bottom} stroke={CHART_COLOR.cursor} strokeOpacity={0.5} />
          <circle cx={cursor.cx} cy={cursor.cy} r={7} fill="none" stroke={CHART_COLOR.cursor} strokeWidth={2} />
        </g>
      )}
    </svg>
  );
}
