/**
 * Differential run for the layout data of flowcharts: every flowchart input goes through the
 * reference's `getData()` (`export-flowchart-data.ts`) and through ferrigura's `getData` (the
 * `flowchart_data` example of `ferrigura-render`). The run itself is in `layout-data-diff.ts`.
 *
 * Usage: `pnpm port:diff flowchart-data`
 */
import { runLayoutDataDiff } from './layout-data-diff.ts';

await runLayoutDataDiff({
  name: 'flowchart-data',
  type: 'flowchart',
  exporter: 'tools/port/export-flowchart-data.ts',
  example: 'flowchart_data',
});
