/**
 * Named exceptions of the differential run: inputs where the reference is wrong, and where the
 * port follows the intent rather than the reference. Each entry names the input by its text and
 * says why it differs. The run checks every entry: an exception that now agrees is an error, so
 * the list cannot outlive the difference it records.
 */
export interface PortException {
  /** The exact text of the input. */
  text: string;
  /** Why the outputs differ, and which side is right. */
  reason: string;
}

export const exceptions: Record<string, PortException[]> = {
  flowchart: [],
  'flowchart-data': [],
  gantt: [],
};
