/** Mermaid renderer configuration. Engine-specific values are validated by Ferrigura. */
export type Config = Readonly<Record<string, unknown>>;
export type RenderOptions = {
  /** SVG namespace; starts with an ASCII letter and contains letters, digits, - or _. */
  id?: string;
  colorScheme?: 'light' | 'dark' | 'auto';
  /** Flowcharts only. `adaptive` centers ports and draws label backgrounds at full opacity;
   * `mermaid` preserves the reference's routing. */
  flowchartMode?: 'adaptive' | 'mermaid';
};
/** Render Mermaid source to an SVG string. Throws on invalid source or configuration. */
export declare function render(text: string, config?: Config, options?: RenderOptions): string;
/** Reuse the resolved configuration across synchronous diagram renders. */
export declare class Renderer {
  constructor(config?: Config);
  render(text: string, options?: RenderOptions): string;
}
