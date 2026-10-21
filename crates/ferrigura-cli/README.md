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
      --id <ID>           The id of the SVG element, which scopes its styles
      --scheme <SCHEME>   The colours: `light`, `dark`, or `auto` to follow the viewer's
                          colour scheme. By default the configured theme decides.
  -o, --output <FILE>     Write the SVG to FILE instead of standard output (one input only)
      --output-dir <DIR>  Write each SVG to DIR/<name>.svg, named after its input file
                          (file inputs only)
  -h, --help              Print this help
  -V, --version           Print the version
```

Without a `FILE`, the diagram text is read from standard input. `-` also means standard input.

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

Read from standard input, and follow the viewer's colour scheme:

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

## Behaviour

- Several inputs are rendered through one renderer, so the configuration is resolved once. They are rendered in the order given.
- Rendering stops at the first input that fails. The SVGs of the inputs before it are already written.
- With several inputs, an error message starts with the name of the input that failed.
- Each SVG ends with a newline. With several inputs on standard output, the SVGs follow each other with no separator.
- `--output` takes one input. `--output-dir` takes file inputs only, and two inputs with the same file name are refused.
- `-` can be given once.

## Exit codes

| Code | Meaning                                                                                                                              |
| ---: | ------------------------------------------------------------------------------------------------------------------------------------ |
|    0 | Success, and also `--help` and `--version`.                                                                                          |
|    1 | A diagram or the configuration does not render, or a file cannot be read or written.                                                 |
|    2 | The command line does not parse: an unknown option, an option without its value, an unknown colour scheme, or options that conflict. |
