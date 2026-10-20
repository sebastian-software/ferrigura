/**
 * Canonical JSON of flowchart diagrams, from the TypeScript reference.
 *
 * The model is the flowchart DB's state after the parse, read by `flowchartModel`
 * (`flowchart-model.ts`), which the swimlane exporter shares.
 *
 * Usage: `tsx tools/port/export-flowchart.ts inputs.json outputs.json`
 */
import { flowchartModel, loadFlowchartModules } from './flowchart-model.ts';
import { runExporter } from './exporter.ts';

await runExporter(loadFlowchartModules, (input, modules) => flowchartModel(input.text, modules));
