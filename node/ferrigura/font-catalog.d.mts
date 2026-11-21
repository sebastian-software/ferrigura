import type { FontOptions } from './index.mjs';

/** A reviewed immutable catalog and the sole origin allowed for its manifest and assets. */
export type FontCatalog = {
  manifestUrl: string;
  manifestSha256: string;
  assetBase: string;
};
/** A catalog family, optionally registered under a different local name. */
export type CatalogFamily = string | { family: string; alias: string };
/** Host-side catalog fetch and verified content-addressed cache settings. */
export type CatalogFontOptions = {
  families: readonly CatalogFamily[];
  cacheDir: string;
  /** Explicit ordered aliases for fallback; defaults to none. */
  fallbacks?: readonly string[];
  offline?: boolean;
  /** Omit to use the pinned public catalog. */
  catalog?: FontCatalog;
};
/** Resolve selected faces before rendering. */
export declare function resolveCatalogFonts(options: CatalogFontOptions): Promise<FontOptions>;
