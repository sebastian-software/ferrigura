# Font catalog candidate

This catalog describes Lexica Ultralegible 1.0.0 in four styles. Each renderer TTF and browser WOFF2 comes from the pinned upstream commit, with its full SHA-256, byte length and OFL 1.1 license. The four TTF files match the current bundled Lexica faces. The styles do not all cover the same characters; hosts should check their own text and provide explicit fallbacks.

Paths in the catalog are relative to `https://assets.sebastian-software.com/ferrigura/`. The content hash is part of every file path, including the license. The catalog itself is written to `catalogs/lexica-ultralegible/1.0.0/<sha256>.json`. A consumer needs a reviewed catalog path and SHA-256; there is no mutable `latest` alias. Resolve a path against that origin, download the file in the host or build pipeline, and check its full SHA-256 and length before registering the local TTF with Ferrigura. A browser can load the paired WOFF2 through `@font-face`; Ferrigura itself does not fetch fonts.

## Checks and publication

```sh
node --test tools/font-catalog/catalog.test.mjs
node tools/font-catalog/run.mjs prepare
node tools/port/fonts/catalog-browser.mjs
```

Preparation downloads the pinned source into `target/font-catalog` only after checking each file's hash and length. The browser check requires the port harness dependencies and Chromium. It loads each TTF/WOFF2 pair from a separate local origin, compares sample measurements, and proves that a request without CORS is rejected. This local check does not establish deployed headers.

A maintainer can manually run the Font catalog workflow with `publish` selected on main. It uses the shared organization `BUNNY_API_KEY` for read-only management lookup, checks that the named shared pull zone serves `assets.sebastian-software.com` from the named shared storage zone, and uses the returned storage password only within the process. It writes only under `ferrigura/` and does not configure or purge the zone. It reads before writing and refuses different bytes at an existing path; it reads every upload back and verifies every public object's bytes and MIME type. Browser WOFF2s must also have wildcard CORS and one-year immutable caching. Host-fetched TTF, JSON and license responses need no browser CORS or cache promise. A failed lookup, upload or public check fails the workflow. Keep successful objects for old catalog versions. The workflow does not publish on a pull request.

The catalog does not instrument downloads. The separate app-tracking service has an opt-in verified-download API, but a catalog host or renderer does not report usage implicitly.
