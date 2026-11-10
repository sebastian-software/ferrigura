/** Mermaid renderer configuration. Engine-specific values are validated by Ferrigura. */
export type Config = Readonly<Record<string, unknown>>;
/** One-time, script-free SVG draw animation for flowchart, Gantt, Git graph, mind map, pie, and XY. */
export type Animation = {
  /** Positive, finite multiplier of 700 SVG units per second on average. Defaults to 1.
   * Each connection, bar, or sector eases in and out independently. */
  speed?: number;
  /** One line at a time in source order (default), or parallel branches. */
  order?: 'source' | 'flow';
};
export type RenderOptions = {
  /** True selects Mermaid geometry. False (default) enables optimized routing and compact participants. */
  compat?: boolean;
  /** An empty object enables draw animation with the defaults. Omit to keep static output. */
  animation?: Animation;
  /** SVG namespace; starts with an ASCII letter and contains letters, digits, - or _. */
  id?: string;
  colorScheme?: 'light' | 'dark' | 'auto';
};
/** One explicit font face. File paths are read by JavaScript, relative to the process directory. */
export type FontFace = {
  family: string;
  /** Integer CSS weight from 1 through 1000; defaults to 400. */
  weight?: number;
  style?: 'normal' | 'italic';
} & ({ data: Uint8Array; path?: never } | { path: string; data?: never });
/** Font registration belongs to renderer initialization, not per-render SVG options. */
export type FontOptions = {
  fonts?: readonly FontFace[];
  /** Ordered fallback family names. Every family must be explicitly registered. */
  fallbacks?: readonly string[];
};
/** Render Mermaid source to an SVG string. Throws on invalid source or configuration. */
export declare function render(
  text: string,
  config?: Config,
  options?: RenderOptions,
  fonts?: FontOptions
): string;
/** Reuse the resolved configuration across synchronous diagram renders. */
export declare class Renderer {
  constructor(config?: Config, fonts?: FontOptions);
  render(text: string, options?: RenderOptions): string;
}
