/**
 * Canonical JSON of swimlane diagrams, from the TypeScript reference.
 *
 * A swimlane diagram is the flowchart's factory with its own styles
 * (`packages/mermaid/src/diagrams/swimlanes/swimlanesDiagram.ts`), so its parser and its DB are the
 * flowchart's, and the model is the flowchart's (`flowchart-model.ts`). The text must first be
 * detected as a swimlane diagram, as `Diagram.fromText` does: `detectType` runs on the cleaned text
 * and its detector (`/^\s*swimlane-beta\b/`, `detector.ts`) decides. A text of another type is a
 * rejection, as it is on the Rust side (`examples/canonical.rs`, `swimlanes_outcome`).
 *
 * Usage: `tsx tools/port/export-swimlanes.ts inputs.json outputs.json`
 */
import { flowchartModel, loadFlowchartModules, type FlowchartModules } from './flowchart-model.ts';
import { runExporter } from './exporter.ts';
import type { Reference } from './reference.ts';

interface Modules extends FlowchartModules {
  detectType: { detectType(text: string): string };
}

async function loadModules(reference: Reference): Promise<Modules> {
  const [flowchart, detectType] = await Promise.all([
    loadFlowchartModules(reference),
    reference.load<Modules['detectType']>('/packages/mermaid/src/diagram-api/detectType.ts'),
  ]);
  return { ...flowchart, detectType };
}

await runExporter(loadModules, (input, modules) => {
  const processed = modules.preprocess.preprocessDiagram(input.text);
  let diagramType: string | undefined;
  try {
    diagramType = modules.detectType.detectType(processed.code.cleaned);
  } catch {
    diagramType = undefined;
  }
  if (diagramType !== 'swimlane') {
    throw new Error(`not a swimlane diagram (detected: ${diagramType ?? 'none'})`);
  }
  return flowchartModel(input.text, modules);
});
