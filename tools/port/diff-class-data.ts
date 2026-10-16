/**
 * Differential run for the layout data of class diagrams: every class input goes through the
 * reference's `getData()` (`export-class-data.ts`) and through ferrigura's `getData` (the
 * `class_data` example of `ferrigura-render`). The run itself is in `layout-data-diff.ts`.
 *
 * Usage: `pnpm port:diff class-data`
 */
import { runLayoutDataDiff } from './layout-data-diff.ts';

await runLayoutDataDiff({
  name: 'class-data',
  type: 'class',
  exporter: 'tools/port/export-class-data.ts',
  example: 'class_data',
});
