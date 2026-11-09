import React, { useMemo } from 'react';
import './CpuSparkline.css';

export interface CpuSparklineProps {
  samples: number[];
  width?: number;
  height?: number;
  className?: string;
}

/**
 * CpuSparkline renders a bounded historical trend of CPU utilization percentages.
 *
 * Guarantees:
 * - Operates purely on an in-memory buffer of at most 30 samples.
 * - Handles missing, empty, or single-sample buffers gracefully.
 * - Renders accessible SVG vector graphic with aria metadata.
 * - Does not trigger DOM re-layouts or canvas memory leaks.
 */
export const CpuSparkline: React.FC<CpuSparklineProps> = ({
  samples,
  width = 120,
  height = 36,
  className = '',
}) => {
  // Ensure valid numerical samples clamped between 0 and 100
  const validSamples = useMemo(() => {
    return samples
      .filter((s) => typeof s === 'number' && !isNaN(s) && isFinite(s))
      .map((s) => Math.max(0, Math.min(100, s)));
  }, [samples]);

  const { pathD, areaD, currentPoint } = useMemo(() => {
    if (validSamples.length < 2) {
      return { pathD: '', areaD: '', currentPoint: null };
    }

    const paddingX = 2;
    const paddingY = 4;
    const innerWidth = width - paddingX * 2;
    const innerHeight = height - paddingY * 2;
    const step = innerWidth / (validSamples.length - 1);

    // Map samples (0-100) to SVG coordinates (y inverted: 0 at bottom, 100 at top)
    const points = validSamples.map((val, idx) => {
      const x = paddingX + idx * step;
      const y = paddingY + innerHeight - (val / 100) * innerHeight;
      return { x, y };
    });

    // Build stroke line path
    let linePath = `M ${points[0].x.toFixed(1)} ${points[0].y.toFixed(1)}`;
    for (let i = 1; i < points.length; i++) {
      linePath += ` L ${points[i].x.toFixed(1)} ${points[i].y.toFixed(1)}`;
    }

    // Build closed area path for subtle gradient fill
    const lastX = points[points.length - 1].x.toFixed(1);
    const bottomY = (height - 1).toFixed(1);
    const firstX = points[0].x.toFixed(1);
    const closedArea = `${linePath} L ${lastX} ${bottomY} L ${firstX} ${bottomY} Z`;

    return {
      pathD: linePath,
      areaD: closedArea,
      currentPoint: points[points.length - 1],
    };
  }, [validSamples, width, height]);

  const latestVal = validSamples.length > 0 ? validSamples[validSamples.length - 1] : null;

  return (
    <div
      className={`cpu-sparkline-wrapper ${className}`.trim()}
      role="img"
      aria-label={
        latestVal !== null
          ? `CPU utilization trend across ${validSamples.length} samples, current ${latestVal.toFixed(1)}%`
          : 'CPU utilization history unavailable'
      }
    >
      <svg
        className="cpu-sparkline-svg"
        viewBox={`0 0 ${width} ${height}`}
        width={width}
        height={height}
        preserveAspectRatio="none"
        aria-hidden="true"
      >
        <defs>
          <linearGradient id="cpuSparklineGrad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="var(--dp-accent-primary, #0969da)" stopOpacity="0.35" />
            <stop offset="100%" stopColor="var(--dp-accent-primary, #0969da)" stopOpacity="0.0" />
          </linearGradient>
        </defs>

        {/* Subtle baseline */}
        <line
          x1="0"
          y1={height - 2}
          x2={width}
          y2={height - 2}
          className="cpu-sparkline-baseline"
        />

        {areaD && <path d={areaD} fill="url(#cpuSparklineGrad)" className="cpu-sparkline-area" />}
        {pathD && <path d={pathD} className="cpu-sparkline-path" />}

        {/* Highlight latest sample dot */}
        {currentPoint && (
          <circle
            cx={currentPoint.x}
            cy={currentPoint.y}
            r="2.5"
            className="cpu-sparkline-dot"
          />
        )}
      </svg>
      <span className="cpu-sparkline-caption">
        {validSamples.length} sample{validSamples.length === 1 ? '' : 's'} (max 30)
      </span>
    </div>
  );
};

