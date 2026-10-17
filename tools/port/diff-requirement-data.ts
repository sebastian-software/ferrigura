/**
 * Differential run for the layout data of requirement diagrams: every requirement input goes
 * through the reference's `getData()` (`export-requirement-data.ts`) and through ferrigura's
 * `getData` (the `requirement_data` example of `ferrigura-render`). The run itself is in
 * `layout-data-diff.ts`.
 *
 * Usage: `pnpm port:diff requirement-data`
 */
import { runLayoutDataDiff } from './layout-data-diff.ts';

await runLayoutDataDiff({
  name: 'requirement-data',
  type: 'requirement',
  exporter: 'tools/port/export-requirement-data.ts',
  example: 'requirement_data',
});
