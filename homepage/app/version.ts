// Generated from Cargo.toml and the npm manifest before development, typecheck
// and build (scripts/write-site-data.mjs).
import site from './data/site.json';

export const { version } = site;

/** "0.4" for "0.4.0": the release line the homepage names. */
export const releaseLine = version.split('.').slice(0, 2).join('.');

/** The minimum toolchains, as "22.13" for "22.13.0". */
const short = (value: string) => value.replace(/\.0$/, '');
export const rustVersion = short(site.rustVersion);
export const nodeVersion = short(site.nodeVersion);
