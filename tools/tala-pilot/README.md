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

The overlay leaves the D2 checkout untouched. It passes the original route
points and label placements to the Rust normalizer, which computes the origin
using the solver's label stroke width. The overlay resumes the pinned Go layout
with the resulting coordinates. The test compares the final JSON and SVG
byte for byte. The executable and Go are development tools.

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

## Check the complete native layout against Go

The end-to-end check runs all authored flowcharts in `fixtures/` through
D2's default TALA layout, which tries three seeds. It compares native Rust
node boxes, route points and edge-label centers exactly, then compares
the SVG from the public Ferrigura renderer with the same Go geometry
painted by Ferrigura. An unsupported Rust layout fails the check.

Build the Go bridge as above, then run from the repository root:

```sh
cargo build --locked --features experimental-tala --example tala_pilot
cargo build --locked --manifest-path tools/tala-pilot/rust/Cargo.toml
node tools/tala-pilot/check-end-to-end.mjs \
  --reference /tmp/ferrigura-tala-reference \
  --ferrigura target/debug/examples/tala_pilot \
  --rust tools/tala-pilot/rust/target/debug/ferrigura-tala
```

The command prints a JSON result for every fixture and exits nonzero when
any case differs or fails. For diagnosis, pass `--case` followed by one
fixture filename. The fixed-seed replay still verifies the saved captures
used by the stage fixtures and website.

## Check the upstream correctness suite

The source inventory reads the pinned D2 revision and lists each named
correctness test, fuzz seed test, and example in the TALA engine packages.
The test map names Rust counterparts for the upstream assertions they
exercise. The verifier runs both sides and checks that the named tests
executed and passed. Benchmarks are reported separately.

```sh
node tools/tala-pilot/check-upstream-tests.mjs \
  --d2 /tmp/ferrigura-tala-d2 --verify --json
```

Use `--require-complete` to fail when any correctness declaration is
unmapped. The **TALA parity** GitHub Actions workflow runs the unchanged
Go suite, checks the map with `--require-complete`, and runs the full
end-to-end comparison. The workflow is manually dispatched and publishes
both reports. Its status is independent of the routine repository checks.

## Replay placement and routing stages

The Rust crate tests replay pinned Go checkpoints for the plain sizeless
initializer, graph-distance initializer, orthogonal route balancing and
route simplification. These checkpoints describe isolated stages of the
general solver. The native renderer currently accepts only the bounded
subset documented in the [TALA crate README](../../crates/ferrigura-tala/README.md).

Run the pilot binary with `--initialize` or `--initialize-by-distance` and
an already preprocessed measured graph on stdin. The graph-distance stage
returns JSON `null` when its input takes the ordinary initialization path.
Pass `--balance` or `--simplify` a JSON object with `graph` and `layout`
entries containing the measured graph and an already routed layout. With no
argument the pilot replays terminal coordinate normalization.

### Adapter translation and prescale

The native renderer uses the ordered adapter graph for validation and
container topology. The Rust replay compares its stable entity IDs,
depth-first node order, edge order, shape and label measurements, and
layout-time edge orientation with the pinned Go adapter. The Go trace
runs prescale on a graph clone and checks that tracing leaves the
unmodified default layout unchanged.

```sh
node tools/tala-pilot/trace-overlay.mjs /tmp/ferrigura-tala-d2 /tmp/ferrigura-tala-graph layout-graph
GOFLAGS=-overlay=/tmp/ferrigura-tala-graph/overlay.json \
  tools/tala-pilot/reference/build.sh /tmp/ferrigura-tala-d2 /tmp/ferrigura-tala-traced-graph
node tools/tala-pilot/check-layout-graph.mjs \
  --reference /tmp/ferrigura-tala-reference \
  --trace /tmp/ferrigura-tala-traced-graph \
  --rust tools/tala-pilot/rust/target/debug/ferrigura-tala
```

Pass `--write` to refresh the two checked-in Rust checkpoints from the
live trace after the comparison passes.

### Seeded sizeless placement

The Rust stage initializes a flat rectangular component, then optimizes
its positions using Go-compatible shuffles and jitter and alternating
compactions. The Observatory check compares every ordered grid location
with Go after the first pass and the complete sizeless phase for all
three default seeds. The check also compares the size-aware transition,
the first move search and the complete first sized pass. It repeats both
the transition and the complete first pass from Ferrigura's measured
input, including native translation, prescale, and the component's
breadth-first placement order. Node coordinates and candidate order match
exactly. Internal edge costs allow floating-point rounding far below Go's
score comparison threshold. The trace preserves the final Go layout.

```sh
node tools/tala-pilot/trace-overlay.mjs \
  /tmp/ferrigura-tala-d2 /tmp/ferrigura-tala-sizeless sizeless-placement
GOFLAGS=-overlay=/tmp/ferrigura-tala-sizeless/overlay.json \
  tools/tala-pilot/reference/build.sh \
  /tmp/ferrigura-tala-d2 /tmp/ferrigura-tala-sizeless-reference
node tools/tala-pilot/check-sizeless-placement.mjs \
  /tmp/ferrigura-tala-reference /tmp/ferrigura-tala-sizeless-reference \
  tools/tala-pilot/rust/target/debug/ferrigura-tala
```

### Authored routes and edge labels

The native rectangle-loop paths use the same projection as the general
adapter for completed polylines. It restores source-only edge traversal,
mirrors the label anchor and unlocked fraction, and drops redundant route
points before calculating the label center used by Ferrigura.

This check feeds the pinned Go winner's routes to that Rust projection
for all measured flowcharts and repeats each with a source-only arrow.
The observer leaves the Go layout output unchanged.

```sh
node tools/tala-pilot/trace-overlay.mjs /tmp/ferrigura-tala-d2 /tmp/ferrigura-tala-route-output route-output
GOFLAGS=-overlay=/tmp/ferrigura-tala-route-output/overlay.json \
  tools/tala-pilot/reference/build.sh /tmp/ferrigura-tala-d2 /tmp/ferrigura-tala-traced-output
node tools/tala-pilot/check-route-output.mjs \
  --reference /tmp/ferrigura-tala-reference \
  --trace /tmp/ferrigura-tala-traced-output \
  --rust tools/tala-pilot/rust/target/debug/ferrigura-tala
```

Pass `--write` to refresh the checked-in route projection fixtures after
the comparison passes.

### Flat orthogonal visibility graph

The Observatory replay compares all stages of the flat visibility graph
from ordered rectangle ports to perimeter and halfway points, boundary
layers, tunnel links, joined row and column links, and center links. Vertex
storage order is compared within stages where Go fixes it. The final
comparison matches variable-order vertices by coordinates and center-owner
identity and compares the ordered links and port metadata exactly.
The traced runner's final output must match independent Go byte for byte.

```sh
node tools/tala-pilot/trace-overlay.mjs /tmp/ferrigura-tala-d2 /tmp/ferrigura-tala-ovg ovg-seed
GOFLAGS=-overlay=/tmp/ferrigura-tala-ovg/overlay.json \
  tools/tala-pilot/reference/build.sh /tmp/ferrigura-tala-d2 /tmp/ferrigura-tala-traced-ovg
node tools/tala-pilot/check-ovg-seed.mjs \
  /tmp/ferrigura-tala-reference /tmp/ferrigura-tala-traced-ovg \
  tools/tala-pilot/rust/target/debug/ferrigura-tala
```

### Completed candidate scoring and seed selection

The candidate-score trace observes each completed ordinary Go attempt before
the public engine considers optional placement and route refinements. The
Rust scorer receives positioned geometry and resolved decoration boxes.
The check compares exact penalty, unrounded area, work consumed and the
winning seed. It checks that the traced runner produces the same complete
layout as an independent uninstrumented Go runner.

```sh
node tools/tala-pilot/trace-overlay.mjs /tmp/ferrigura-tala-d2 /tmp/ferrigura-tala-score candidate-score
GOFLAGS=-overlay=/tmp/ferrigura-tala-score/overlay.json \
  tools/tala-pilot/reference/build.sh /tmp/ferrigura-tala-d2 /tmp/ferrigura-tala-traced-score
node tools/tala-pilot/check-candidate-scores.mjs \
  --reference /tmp/ferrigura-tala-reference \
  --traced /tmp/ferrigura-tala-traced-score \
  --rust tools/tala-pilot/rust/target/debug/ferrigura-tala
```

Pass `--write-fixtures crates/ferrigura-tala/src/quality/go-candidates.json`
to regenerate the cases shared with the Rust test suite.

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
