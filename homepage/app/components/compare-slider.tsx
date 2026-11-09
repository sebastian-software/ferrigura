import type { CSSProperties, ReactNode } from 'ardo/react';

import { ReactCompareSlider, ReactCompareSliderHandle } from 'react-compare-slider';

/** The handle in the page's accent, on the sheet's color, so it shows on a white drawing. */
const handleStyle: CSSProperties & Record<'--rcs-handle-color', string> = {
  '--rcs-handle-color': 'var(--mono-oxide)',
};
const buttonStyle: CSSProperties = {
  backgroundColor: 'var(--mono-sheet)',
  boxShadow: '0 1px 4px rgba(38, 35, 31, 0.25)',
};

export type CompareSliderProps = {
  /** Shown left of the divider. */
  first: ReactNode;
  firstLabel: string;
  /** Shown right of the divider. */
  second: ReactNode;
  secondLabel: string;
};

/**
 * Two pictures of the same size laid over each other, split by a divider:
 * the first shows left of it, the second right of it. react-compare-slider
 * moves the divider by pointer, touch and the arrow keys; on a touch screen
 * only the handle moves it, so a swipe elsewhere still scrolls the page.
 */
export function CompareSlider({ first, firstLabel, second, secondLabel }: CompareSliderProps) {
  return (
    <div className="fgr-compare">
      <div className="fgr-compare-tags" aria-hidden="true">
        <span>← {firstLabel}</span>
        <span>{secondLabel} →</span>
      </div>
      <ReactCompareSlider
        className="fgr-compare-stage"
        itemOne={<div className="fgr-compare-layer">{first}</div>}
        itemTwo={<div className="fgr-compare-layer">{second}</div>}
        keyboardIncrement="2%"
        handle={<ReactCompareSliderHandle style={handleStyle} buttonStyle={buttonStyle} />}
      />
    </div>
  );
}
