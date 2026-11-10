# ferrigura-cli

`ferrigura-cli` is the command-line tool of [ferrigura](https://github.com/sebastian-software/ferrigura). It renders Mermaid diagram files to SVG, without a browser. The binary is called `ferrigura`.

## Installation

```sh
cargo install ferrigura-cli
```

This installs the `ferrigura` binary. To install it from a checkout of the repository:

```sh
cargo install --path crates/ferrigura-cli
```

## Usage

```text
Renders Mermaid diagrams to SVG.

Usage: ferrigura [OPTIONS] [FILE]...

Arguments:
  [FILE]...  The diagram texts. Standard input is read when no FILE is given, or for `-`. Several
             files are rendered through one renderer, in the order given. Without --output or
             --output-dir, their SVGs follow each other on standard output.

Options:
  -c, --config <FILE>     The configuration as a JSON object, what `initialize` takes
      --font <FAMILY=FILE> Register a local font as weight 400, normal; repeat for more families
      --font-face <FAMILY:WEIGHT:STYLE=FILE>
                          Register a face with weight 1..1000 and style normal or italic
      --font-fallback <FAMILY>
                          Add a registered fallback family, in order; repeat for more families
      --id <ID>           The id of the SVG element, which scopes its styles
      --scheme <SCHEME>   The colours: `light`, `dark`, or `auto` to follow the viewer's
                          colour scheme. By default the configured theme decides.
      --compat           Use Mermaid geometry instead of optimized routing and participant boxes
      --animate           Draw a supported diagram once, one line at a time, with embedded CSS
      --animation-speed <N>
                          Positive multiplier of 700 SVG units/s on average; enables animation
      --animation-order <ORDER>
                          `source` (default) or `flow` for parallel branches; enables animation
  -o, --output <FILE>     Write the SVG to FILE instead of standard output (one input only)
      --output-dir <DIR>  Write each SVG to DIR/<name>.svg, named after its input file
                          (file inputs only)
  -h, --help              Print this help
  -V, --version           Print the version
```

Without a `FILE`, the diagram text is read from standard input. `-` also means standard input.

## Custom fonts

Register a named local file before rendering; no system fonts or network sources
are discovered:

```sh
ferrigura --font "Project Sans=./fonts/regular.ttf" diagram.mmd > diagram.svg
```

`--font` is repeatable and defaults to weight 400 and normal style. The first
registered family is the default unless `--config` explicitly sets `fontFamily`.
Use `--font-face "Project Sans:600:italic=./fonts/italic.ttf"` for a face with an
integer CSS weight from 1 through 1000 and normal or italic style.
Paths are relative to the process directory and can contain `:` or `=`.

Register fallback faces with `--font` or `--font-face`, then give their names in
order with repeatable `--font-fallback "Symbols"`. When custom fonts are used,
every requested or fallback family must be explicitly registered. A fallback
can supply an entire label. Font bytes are loaded and validated before input is
read or output is created; an unreadable or invalid file exits with status 1.
Malformed font flags exit with status 2.

Mermaid configuration, front matter, and init directives can select registered
family names, but cannot register paths or URLs. SVG output continues to embed
its used font subsets. Without these flags, bundled defaults are unchanged.

## Examples

Render a file to standard output:

```sh
ferrigura pets.mmd > pets.svg
```

Write the SVG to a file:

```sh
ferrigura -o pets.svg pets.mmd
```

Render several files into one directory, each to `<name>.svg`:

```sh
ferrigura --output-dir out/ pets.mmd cats.mmd
```

Read from standard input, and follow the viewer's color scheme:

```sh
cat pets.mmd | ferrigura --scheme auto > pets.svg
```

Give the configuration as a file that holds a JSON object, the object that `initialize` takes:

```sh
ferrigura --config config.json --id pets pets.mmd
```

```json
{ "themeVariables": { "pie1": "#00ff00" } }
```

## Behavior

`--animate` enables a one-time draw effect with speed `1` and one line
at a time in source order. `--animation-speed 1.5` multiplies drawing velocity;
it does not set the whole diagram's duration. Each connection, bar, or sector
eases in and out independently. `--animation-order flow` draws
branches in parallel. Either setting enables animation, so `--animate` is
optional when a speed or order is supplied. Invalid speed or order values are
usage errors. Flowchart, Gantt, Git graph, mind map, pie (including donut), and
XY chart support this option; other diagram types return a render error. For
charts, `flow` starts independent elements together. See
[draw animation](../../docs/animation.md) for each type's behavior.

The SVG contains CSS and masks without JavaScript. Reduced motion shows the
complete diagram. Without animation options, output remains unchanged.

By default, optimized rendering centers flowchart ports and compacts sequence participant boxes. `--compat` preserves Mermaid-compatible geometry across diagram types. Font, fill and readability fixes apply in both modes.

- Several inputs are rendered through one renderer, so the configuration is resolved once. They are rendered in the order given.
- Rendering stops at the first input that fails. The SVGs of the inputs before it are already written.
- With several inputs, an error message starts with the name of the input that failed.
- Each SVG ends with a newline. With several inputs on standard output, the SVGs follow each other with no separator.
- `--output` takes one input. `--output-dir` takes file inputs only, and two inputs with the same file name are refused.
- `-` can be given once.

## Exit codes

| Code | Meaning                                                                                                                             |
| ---: | ----------------------------------------------------------------------------------------------------------------------------------- |
|    0 | Success, and also `--help` and `--version`.                                                                                         |
|    1 | A diagram or the configuration does not render, or a file cannot be read or written.                                                |
|    2 | The command line does not parse: an unknown option, an option without its value, an unknown color scheme, or options that conflict. |
