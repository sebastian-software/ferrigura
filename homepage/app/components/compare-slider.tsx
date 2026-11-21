import type { CSSProperties, ReactNode } from 'ardo/react';

import { Handle, HandleRoot, Item, Provider, Root } from 'react-compare-slider/components';
import { useReactCompareSlider } from 'react-compare-slider/hooks';

/** The handle in the page's accent, on the sheet's color, so it shows on a white drawing. */
const handleStyle: CSSProperties & Record<'--rcs-handle-color', string> = {
  '--rcs-handle-color': 'var(--mono-oxide)',
};
const buttonStyle: CSSProperties = {
  backgroundColor: 'var(--mono-sheet)',
  boxShadow: '0 1px 4px rgba(38, 35, 31, 0.25)',
};

export type CompareSliderProps = {
  label: string;
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
export function CompareSlider({
  label,
  first,
  firstLabel,
  second,
  secondLabel,
}: CompareSliderProps) {
  const slider = useReactCompareSlider({ keyboardIncrement: '2%' });
  return (
    <div className="fgr-compare">
      <div className="fgr-compare-tags" aria-hidden="true">
        <span>← {firstLabel}</span>
        <span>{secondLabel} →</span>
      </div>
      <Provider {...slider}>
        <Root className="fgr-compare-stage">
          <Item item="itemOne">
            <div className="fgr-compare-layer">{first}</div>
          </Item>
          <Item item="itemTwo">
            <div className="fgr-compare-layer">{second}</div>
          </Item>
          <HandleRoot
            aria-label={`${label}: ${firstLabel} left of the divider, ${secondLabel} right of it`}
          >
            <Handle style={handleStyle} buttonStyle={buttonStyle} />
          </HandleRoot>
        </Root>
      </Provider>
    </div>
  );
}
