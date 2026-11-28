/** Mermaid renderer configuration. Engine-specific values are validated by Ferrigura. */
export type Config = Readonly<Record<string, unknown>>;
/** One-time, script-free SVG draw animation for flowchart, Gantt, mind map, pie, and XY. */
export type Animation = {
  /** Positive, finite multiplier of 700 SVG units per second on average. Defaults to 1.
   * Each connection, bar, or sector eases in and out independently. */
  speed?: number;
  /** One line at a time in source order (default), or parallel branches. */
  order?: 'source' | 'flow';
};
export type RenderOptions = {
  /** True draws Mermaid's geometry and look. False (default) draws ferrigura's own look and measures for every type. */
  compat?: boolean;
  /** Explicit layout selection. Experimental TALA needs a TALA-enabled build. */
  layoutEngine?: 'elk' | 'tala';
  /** An empty object enables draw animation with the defaults. Omit to keep static output. */
  animation?: Animation;
  /** SVG namespace; starts with an ASCII letter and contains letters, digits, - or _. */
  id?: string;
  colorScheme?: 'light' | 'dark' | 'auto';
  /** UTC milliseconds since the Unix epoch, an integer within ±8640000000000000. Defaults to the system clock. */
  now?: number;
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
  /** Ordered fallback names, resolved only against registered faces. */
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
