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
  'flowchart-data': [
    {
      text: "---\ndescription:\n  Verifies that the flowchart rankSpacing configuration option controls the\n  vertical spacing between ranks, under the hand-drawn look.\nconfig:\n  look: handDrawn\n  flowchart:\n    rankSpacing: '100'\n  fontFamily: courier\n---\ngraph TD\n  A[Christmas] -->|Get money| B(Go shopping)\n  B --> C{Let me think}\n  %% this is a comment\n  C -->|One| D[Laptop]\n  C -->|Two| E[iPhone]\n  C -->|Three| F[fa:fa-car Car]\n",
      reason:
        "The typed configuration rejects `rankSpacing: '100'`, a string, because the schema's type is a number. The reference keeps the string and renders; getData does not read rankSpacing, and the layout ignores it for ELK (the getData notes of M5, section 6). The coercion of configuration values belongs to the configuration port.",
    },
    {
      text: "---\ndescription:\n  Verifies that the `flowchart.rankSpacing` config option, provided here as the\n  string value '100', controls the vertical spacing between ranks in a top-down\n  flowchart.\nconfig:\n  flowchart:\n    rankSpacing: '100'\n  fontFamily: courier\n---\ngraph TD\n  A[Christmas] -->|Get money| B(Go shopping)\n  B --> C{Let me think}\n  %% this is a comment\n  C -->|One| D[Laptop]\n  C -->|Two| E[iPhone]\n  C -->|Three| F[fa:fa-car Car]\n",
      reason:
        "The typed configuration rejects `rankSpacing: '100'`, a string, because the schema's type is a number. The reference keeps the string and renders; getData does not read rankSpacing, and the layout ignores it for ELK (the getData notes of M5, section 6). The coercion of configuration values belongs to the configuration port.",
    },
  ],
};
