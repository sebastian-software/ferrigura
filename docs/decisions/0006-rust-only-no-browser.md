# 0006 Rust only, no browser at run time

**Scope:** how ferrigura runs and what it outputs: the library, the command-line tool and the Node.js addon. It does not cover the harness, which runs the reference in Node.js and Chromium to check ferrigura ([0015](./0015-differential-testing.md)).

## Decision

ferrigura renders diagram text to SVG in Rust, without a browser or a JavaScript engine. It ships as a Rust library, a command-line tool and a native Node.js addon built with napi-rs ([0016](./0016-npm-packages-via-napi.md)). There is no browser build and no WebAssembly build. The output is SVG only.

## Why

Rendering happens where diagram text is processed: servers, build pipelines, documentation tools. Without a browser, ferrigura calculates every size itself instead of asking the DOM, so the same text gives the same SVG on every machine, in-process and fast.

Weighed and not chosen:

- **WebAssembly behind Mermaid's TypeScript API:** keeps browser rendering, but either keeps the DOM for measurement or ships fonts and a text engine to every page.
- **A Rust core inside a JavaScript shell:** two implementations to maintain for good.
- **Command-line binaries distributed through npm** (as esbuild does): simpler to build, but a process per call, where the addon is called in-process.

## Consequences

- The programming interface is ferrigura's own and small: a `Renderer` made once for a configuration with `render(text)`, and `render(text, config)` as a shorthand. Mermaid's JavaScript API (`initialize`, `run`, `parse`, callbacks) does not exist.
- Interactivity goes: links stay SVG `<a>`, tooltips become SVG `<title>`, JavaScript callbacks are dropped, and `securityLevel` comes down to escaping text safely.
- PNG and other raster formats are out of scope; any SVG rasterizer can produce them from the output.

## Related

[0009](./0009-svg-text-labels.md), [0010](./0010-text-measurement-and-fonts.md), [0016](./0016-npm-packages-via-napi.md).
