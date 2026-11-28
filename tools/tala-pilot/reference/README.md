# TALA Go reference bridge

The bridge passes Ferrigura's measured flowchart to D2's public TALA layout API as an in-memory graph. By default the bridge uses seed `1` and one worker. Pass `--seed N` to request a fixed seed or `--default` to run D2's default seed selection. It does not parse D2 source or render D2 SVGs.

## Build and verify

Use Go 1.27.4 and a clean D2 checkout at [D2 v0.10.0](https://github.com/d2lang/d2/releases/tag/v0.10.0), commit `b463ea5d47272f4da6e1e08012dbb15a6a4dbcde`. The build uses the pinned checkout's `go.mod` and `go.sum` in read-only mode. Go may need to fetch that module's dependencies on the first build. An existing Go module cache allows an offline build with `GOPROXY=off`.

From the Ferrigura repository root:

```sh
tools/tala-pilot/reference/build.sh /path/to/d2 /tmp/ferrigura-tala-reference
/tmp/ferrigura-tala-reference < measured.json > tala.json
/tmp/ferrigura-tala-reference < measured.json > tala-repeat.json
cmp tala.json tala-repeat.json
```

With an absolute Ferrigura path, run the bridge tests from the pinned D2 checkout:

```sh
GOTOOLCHAIN=local go test -mod=readonly \
  /path/to/ferrigura/tools/tala-pilot/reference/main.go \
  /path/to/ferrigura/tools/tala-pilot/reference/main_test.go
```

After generating the comparison, replay its captured inputs and verify the box dimensions, containment, routes, and labels:

```sh
node tools/tala-pilot/reference/verify.mjs /tmp/ferrigura-tala-reference /path/to/comparison
```

Go is required when generating or validating the reference data. Ferrigura can read the resulting JSON without Go.

## Geometry contract

Input has `direction`, ordered `nodes` and ordered `edges`. Each node carries its original ID, measured width and height, and may carry a parent ID, direction, shape, group flag and measured label. Each edge carries its original ID, source and target IDs, arrow flags and optional measured label. An omitted arrow flag means there is no arrow at that end.

The JSON result preserves input IDs and ordering. Node coordinates are absolute top-left positions. Edge points and label centers are absolute, with positive Y pointing down. Label centers come from D2's own route-label calculation. The result records the dimensions D2 used for each label. Input text dimensions round up to the next integer because D2 stores label dimensions as integers. The bridge fixes the D2 node font size so that the layout does not apply D2's automatic font-size scaling to Ferrigura's measured text.

The adapter encodes node IDs as a letter followed by the UTF-8 bytes in hex so D2's case-insensitive, dot-separated keys preserve a one-to-one mapping with Ferrigura IDs. Edges use their zero-based input index within D2. Rectangles, diamonds, circles, ovals, cylinders and hexagons map to the corresponding D2 shapes. Groups use rectangles. Before packing children, D2 requires a positive initial group box. If Ferrigura hasn't calculated the group box yet, the bridge uses at least 1 px per axis and enough room for the title. D2 treats a group with no children as a rectangle.

TALA may expand measured leaf boxes as part of its own layout. In the Workshop fixture it increases the height of the sensor and panel from 56.4 and 41 px to 120 px. Its port spacing uses two parallel connections and a 40 px gap. The capture preserves that expansion and the original text measurements. [TALA prescale at the pin](https://github.com/d2lang/d2/blob/b463ea5d47272f4da6e1e08012dbb15a6a4dbcde/d2layouts/d2talalayout/internal/placement/stages.go#L42-L85)
