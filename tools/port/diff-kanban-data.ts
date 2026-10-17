/**
 * Differential run for the layout data of kanban diagrams: every kanban input goes through the
 * reference's `getData()` (`export-kanban-data.ts`) and through ferrigura's `getData` (the
 * `kanban_data` example of `ferrigura-render`). The run itself is in `layout-data-diff.ts`.
 *
 * Usage: `pnpm port:diff kanban-data`
 */
import { runLayoutDataDiff } from './layout-data-diff.ts';

await runLayoutDataDiff({
  name: 'kanban-data',
  type: 'kanban',
  exporter: 'tools/port/export-kanban-data.ts',
  example: 'kanban_data',
});
