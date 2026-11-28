# TALA layout pilot

The pilot measures original Mermaid flowcharts with Ferrigura, lays them out
with ELK Layered and pinned Go TALA, and paints the results with the same SVG
renderer. The checked-in captures use Ferrigura’s default look and SVG text
measurements. The pilot substitutes the Rust port of TALA’s terminal
coordinate normalization into the Go engine and verifies the final geometry
and SVG against the unmodified Go path. This normalization port covers
unpinned graphs.
[Decision 0025](../../docs/decisions/0025-tala-pilot.md) ·
[Source and license provenance](./PROVENANCE.md)

## Replay offline

Run from the repository root. The saved measurements and Go outputs in
`captures/` cover eight original Mermaid flowcharts. When replaying SVGs,
the comparison checks that the current Ferrigura measurements still match
the captures. The Rust check compares ordered IDs and every node position and
route point with the Go result at the normalization boundary.

```sh
cargo build --offline --locked --example tala_pilot
cargo build --offline --locked --manifest-path tools/tala-pilot/rust/Cargo.toml

node tools/tala-pilot/compare.mjs \
  --ferrigura target/debug/examples/tala_pilot \
  --captures tools/tala-pilot/captures \
  --out /tmp/ferrigura-tala-preview

node tools/tala-pilot/check-normalization.mjs \
  --rust tools/tala-pilot/rust/target/debug/ferrigura-tala \
  --captures tools/tala-pilot/captures/normalization.json
```

Open `/tmp/ferrigura-tala-preview/index.html` for the diagrams, source,
measurements, and raw SVG links.

## Regenerate from Go and check the mixed layout

Use Go 1.27.4 or the toolchain required by the pinned D2 checkout. The runner
uses seed 1, one worker, and the same measured input as the offline replay.

```sh
git clone https://github.com/d2lang/d2.git /tmp/ferrigura-tala-d2
git -C /tmp/ferrigura-tala-d2 checkout --detach b463ea5d47272f4da6e1e08012dbb15a6a4dbcde

tools/tala-pilot/reference/build.sh /tmp/ferrigura-tala-d2 /tmp/ferrigura-tala-reference
node tools/tala-pilot/compare.mjs \
  --ferrigura target/debug/examples/tala_pilot \
  --reference /tmp/ferrigura-tala-reference \
  --out /tmp/ferrigura-tala-preview
node tools/tala-pilot/reference/verify.mjs /tmp/ferrigura-tala-reference /tmp/ferrigura-tala-preview

node tools/tala-pilot/trace-overlay.mjs /tmp/ferrigura-tala-d2 /tmp/ferrigura-tala-trace
GOFLAGS=-overlay=/tmp/ferrigura-tala-trace/overlay.json \
  tools/tala-pilot/reference/build.sh \
  /tmp/ferrigura-tala-d2 /tmp/ferrigura-tala-traced-reference

node tools/tala-pilot/check-normalization.mjs \
  --rust tools/tala-pilot/rust/target/debug/ferrigura-tala \
  --reference /tmp/ferrigura-tala-traced-reference \
  --ferrigura target/debug/examples/tala_pilot \
  --measured /tmp/ferrigura-tala-preview \
  --captures /tmp/ferrigura-tala-preview/normalization.json
```

The overlay leaves the D2 checkout untouched. It runs the Rust executable
in place of Go's normalization and then resumes the pinned Go layout. The
test compares final JSON and the resulting SVG byte for byte. The executable
and Go are development tools.

## Replay native layout fixtures

The Rust crate tests compare native coordinates against the committed Go
captures with no Go installation. To verify both sides against a fresh
pinned bridge, build it as described above, then run:

```sh
cargo build --offline --locked --manifest-path tools/tala-pilot/rust/Cargo.toml
node tools/tala-pilot/check-native.mjs \
  --reference /tmp/ferrigura-tala-reference \
  --rust tools/tala-pilot/rust/target/debug/ferrigura-tala
```

The verifier runs each stored measured graph through both engines. It
compares node boxes, ordered route points, and label positions. The Rust
renderer receives only the native engine's geometry in an opt-in render.

## Findings

- The nested library diagram goes from 10 bends with ELK to 5 with TALA.
  TALA uses a narrower, taller canvas and puts more total length into its
  routes. The shape of the available space matters.
- ELK draws the directional publishing flow with shorter routes in total,
  711 versus 1,120 SVG units. It is the better fit for this example.
- TALA cuts bends and total route length in the cyclic observatory diagram
  but introduces one interior crossing.
- TALA grows the rectangular sensor and panel in the workshop diagram to
  make room for parallel connections. The replay paints their outlines at
  the grown size and retains the measured labels.

The stage and SVG comparisons support continuing the port for grouped
Mermaid flowcharts. The next scope is placement, packing, and orthogonal
routing at the same measured boundary. The grouped and cyclic samples
describe the behavior of the pinned Go engine.
The homepage generator verifies three native Rust controls against their Go
replays: the labeled self-loop, an edge across a container boundary, and a
pair of labeled parallel edges.

## Benchmark a release build

```sh
cargo build --release --locked -p ferrigura --features experimental-tala --example tala_pilot
node tools/tala-pilot/benchmark.mjs --reference /tmp/ferrigura-tala-reference > /tmp/tala-benchmark.json
```

Pass `--reference` with the Go runner built above to time the pinned upstream
layout call. The Rust layout timing includes each engine’s adapter and the
application of its geometry. Parsing, text measurement, SVG painting,
subprocess startup and JSON are outside the timed layout intervals. The Rust
render timing covers the public Rust render method on a reused renderer,
including parse, measurement, layout and SVG painting. Each interval runs
in its own process and the script records that process’s peak RSS where
`/usr/bin/time` supports it. The default run uses ten warmups and fifty samples.
TALA support failures are recorded with their diagnostics in the JSON.

The [recorded result](./benchmark-results.json) identifies the versions,
hardware, seed and build profile, plus median, p95 and memory for each case.
It records the source revision used for the measured build.
