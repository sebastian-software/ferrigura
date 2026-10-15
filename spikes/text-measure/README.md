# Spike: text measurement without a browser

Measures label text in Rust with the bundled fonts and compares the result with Chromium. Findings: [docs-port/spikes/text-measurement.md](../../docs-port/spikes/text-measurement.md).

| File          | Purpose                                                                                         |
| ------------- | ----------------------------------------------------------------------------------------------- |
| `src/main.rs` | Reads measurement requests as JSON on stdin, writes widths, line metrics and wrapped lines      |
| `corpus.py`   | Builds requests from labels in the e2e fixtures plus edge cases                                 |
| `browser.ts`  | Measures the same requests in Chromium (Playwright) with the same font files                    |
| `compare.py`  | Prints the differences                                                                          |
| `fonts/`      | Lexica Ultralegible and Atkinson Hyperlegible Mono, SIL Open Font License (see the `OFL-*.txt`) |

Run from this folder:

```bash
python3 corpus.py ../.. > /tmp/requests.json
```

```bash
cargo +stable run --release < /tmp/requests.json > /tmp/rust.json
```

```bash
../../node_modules/.bin/tsx browser.ts /tmp/requests.json > /tmp/browser.json
```

```bash
python3 compare.py /tmp/requests.json /tmp/rust.json /tmp/browser.json
```

The crate is standalone (it declares its own empty `[workspace]`), so it does not join the ferrigura workspace. It is a spike: M2 turns what it learned into `ferrigura-text`.
