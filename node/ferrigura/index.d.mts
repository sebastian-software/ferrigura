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
  /** An empty object enables draw animation with the defaults. Omit to keep static output. */
  animation?: Animation;
  /** SVG namespace; starts with an ASCII letter and contains letters, digits, - or _. */
  id?: string;
  colorScheme?: 'light' | 'dark' | 'auto';
  /** Flowcharts only. `adaptive` centers ports and draws label backgrounds at full opacity;
   * `mermaid` preserves the reference's routing and half-opacity label backgrounds. */
  flowchartMode?: 'adaptive' | 'mermaid';
};
/** Render Mermaid source to an SVG string. Throws on invalid source or configuration. */
export declare function render(text: string, config?: Config, options?: RenderOptions): string;
/** Reuse the resolved configuration across synchronous diagram renders. */
export declare class Renderer {
  constructor(config?: Config);
  render(text: string, options?: RenderOptions): string;
}
