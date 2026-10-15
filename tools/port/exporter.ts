/**
 * Shared run loop of the exporters: reads the inputs, converts each one with the reference and
 * writes the outputs as JSON. Every exporter is a script of its own, run with tsx.
 */
import { readFile, writeFile } from 'node:fs/promises';

import type { PortInput, PortOutput } from './reference.ts';
import { openReference, type Reference } from './reference.ts';

/** Reads `<inputs.json> <outputs.json>` from the arguments and runs `convert` on every input. */
export async function runExporter<Modules>(
  loadModules: (reference: Reference) => Promise<Modules>,
  convert: (input: PortInput, modules: Modules) => Promise<unknown>
): Promise<void> {
  const [inputPath, outputPath] = process.argv.slice(2);
  if (!inputPath || !outputPath) {
    throw new Error('usage: tsx <exporter> <inputs.json> <outputs.json>');
  }
  const inputs = JSON.parse(await readFile(inputPath, 'utf8')) as PortInput[];

  const reference = await openReference();
  try {
    const modules = await loadModules(reference);
    const outputs: PortOutput[] = [];
    for (const input of inputs) {
      try {
        const model = await convert(input, modules);
        outputs.push({ id: input.id, ok: true, model });
      } catch (error) {
        outputs.push({
          id: input.id,
          ok: false,
          error: error instanceof Error ? error.message : String(error),
        });
      }
    }
    await writeFile(outputPath, JSON.stringify(outputs, null, 2));
  } finally {
    await reference.close();
  }
}
