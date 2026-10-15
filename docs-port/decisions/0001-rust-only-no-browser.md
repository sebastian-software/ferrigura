# Rust only, no browser at runtime

ferrigura renders diagram text to SVG in Rust, and ships as a library, a command-line tool and a native Node.js addon built with napi-rs. There is no browser build and no WebAssembly: rendering happens where the text is processed (servers, build pipelines, documentation tools), and dropping the browser lets us calculate every size ourselves instead of asking the DOM. The TypeScript implementation stays only as a reference until each diagram type reaches parity.

## Considered Options

- **WebAssembly behind the existing TypeScript API.** Keeps browser rendering, but either keeps the DOM for measurement or ships fonts and a text engine to every page. Rejected.
- **A permanent hybrid, Rust core inside a JavaScript shell.** Two implementations to maintain forever. Rejected.
- **CLI binaries distributed through npm** (as esbuild does). Simpler to build, but costs a process per call. The napi-rs addon is called in-process.

## Consequences

- Runtime interactivity goes away: links stay as SVG `<a>`, tooltips become `<title>`, JavaScript callbacks and the `sandbox` and `loose` security levels have no meaning.
- Output is SVG only.
