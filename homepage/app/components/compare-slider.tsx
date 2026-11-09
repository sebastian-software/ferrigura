import type { CSSProperties, ReactNode } from 'ardo/react';

import { useState } from 'ardo/react';

export type CompareSliderProps = {
  /** Names the control for assistive technology, e.g. "Class diagram". */
  label: string;
  /** Shown left of the divider. */
  first: ReactNode;
  firstLabel: string;
  /** Shown right of the divider; it sets the figure's height. */
  second: ReactNode;
  secondLabel: string;
  /** Where the divider starts, in percent of the width. */
  initial?: number;
};

/**
 * Two pictures of the same size laid over each other, split by a divider.
 * The first shows left of the divider, the second right of it. A native
 * range input covers the figure, so dragging, tapping, the arrow keys and
 * screen readers move the divider without code of their own; without
 * JavaScript, the divider stays at its start.
 */
export function CompareSlider({
  label,
  first,
  firstLabel,
  second,
  secondLabel,
  initial = 50,
}: CompareSliderProps) {
  const [position, setPosition] = useState(initial);
  const style: CSSProperties & Record<`--${string}`, string> = {
    '--fgr-compare-position': `${position}%`,
  };
  return (
    <div className="fgr-compare" style={style}>
      <div className="fgr-compare-tags" aria-hidden="true">
        <span>← {firstLabel}</span>
        <span>{secondLabel} →</span>
      </div>
      <div className="fgr-compare-stage">
        <div className="fgr-compare-layer">{second}</div>
        <div className="fgr-compare-layer fgr-compare-first">{first}</div>
        <input
          className="fgr-compare-input"
          type="range"
          min={0}
          max={100}
          step={1}
          value={position}
          aria-label={`${label}: ${firstLabel} left of the divider, ${secondLabel} right of it`}
          aria-valuetext={`${firstLabel} ${position}%, ${secondLabel} ${100 - position}%`}
          onChange={(event) => {
            setPosition(Number(event.currentTarget.value));
          }}
        />
        <div className="fgr-compare-divider" aria-hidden="true" />
      </div>
    </div>
  );
}
