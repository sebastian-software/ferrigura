// The reference bridge turns Ferrigura's measured flowchart into an in-memory
// D2 graph and captures the public TALA layout result.
package main

import (
	"context"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"math"
	"os"
	"strconv"
	"strings"
	"time"

	"github.com/d2lang/d2/d2graph"
	"github.com/d2lang/d2/d2layouts/d2talalayout"
	"github.com/d2lang/d2/d2target"
	"github.com/d2lang/d2/lib/geo"
)

const pilotSeed = 1

type graphInput struct {
	Direction string      `json:"direction"`
	Nodes     []nodeInput `json:"nodes"`
	Edges     []edgeInput `json:"edges"`
}

type nodeInput struct {
	ID        string  `json:"id"`
	ParentID  string  `json:"parentId"`
	Direction string  `json:"direction"`
	Shape     string  `json:"shape"`
	IsGroup   bool    `json:"isGroup"`
	Width     float64 `json:"width"`
	Height    float64 `json:"height"`
	textInput
}

type edgeInput struct {
	ID          string `json:"id"`
	Source      string `json:"source"`
	Target      string `json:"target"`
	SourceArrow bool   `json:"sourceArrow"`
	TargetArrow bool   `json:"targetArrow"`
	textInput
}

type textInput struct {
	Label       string  `json:"label"`
	LabelWidth  float64 `json:"labelWidth"`
	LabelHeight float64 `json:"labelHeight"`
}

type point struct {
	X float64 `json:"x"`
	Y float64 `json:"y"`
}

type graphOutput struct {
	Seed  int64        `json:"seed,omitempty"`
	Nodes []nodeOutput `json:"nodes"`
	Edges []edgeOutput `json:"edges"`
}

type nodeOutput struct {
	ID     string  `json:"id"`
	X      float64 `json:"x"`
	Y      float64 `json:"y"`
	Width  float64 `json:"width"`
	Height float64 `json:"height"`
}

type edgeOutput struct {
	ID          string  `json:"id"`
	Points      []point `json:"points"`
	Label       *point  `json:"label,omitempty"`
	LabelWidth  int     `json:"labelWidth,omitempty"`
	LabelHeight int     `json:"labelHeight,omitempty"`
}

func direction(value string) (string, error) {
	switch strings.ToUpper(value) {
	case "":
		return "", nil
	case "TB", "TD":
		return "down", nil
	case "BT":
		return "up", nil
	case "LR":
		return "right", nil
	case "RL":
		return "left", nil
	default:
		return "", fmt.Errorf("unsupported direction %q", value)
	}
}

func shape(value string) (string, error) {
	switch value {
	case "", "rect", "rounded", "rectangle", "square", "squareRect", "round-rect", "roundedRect":
		return d2target.ShapeRectangle, nil
	case "diam", "diamond", "question":
		return d2target.ShapeDiamond, nil
	case "circle", "circ":
		return d2target.ShapeCircle, nil
	case "stadium", "oval", "pill":
		return d2target.ShapeOval, nil
	case "cylinder", "cyl", "db":
		return d2target.ShapeCylinder, nil
	case "hexagon", "hex":
		return d2target.ShapeHexagon, nil
	default:
		return "", fmt.Errorf("unsupported shape %q", value)
	}
}

func measuredDimensions(boxWidth, boxHeight float64) bool {
	return boxWidth >= 0 && boxWidth <= 1_000_000 &&
		boxHeight >= 0 && boxHeight <= 1_000_000
}

func (t textInput) attributes() (d2graph.Attributes, error) {
	if !measuredDimensions(t.LabelWidth, t.LabelHeight) {
		return d2graph.Attributes{}, errors.New("invalid measured label dimensions")
	}
	return d2graph.Attributes{
		Label: d2graph.Scalar{Value: t.Label},
		LabelDimensions: d2target.TextDimensions{
			Width:  int(math.Ceil(t.LabelWidth)),
			Height: int(math.Ceil(t.LabelHeight)),
		},
	}, nil
}

func makeGraph(input graphInput) (*d2graph.Graph, map[string]*d2graph.Object, []*d2graph.Edge, error) {
	graph := d2graph.NewGraph()
	rootDirection, err := direction(input.Direction)
	if err != nil {
		return nil, nil, nil, err
	}
	graph.Root.Direction.Value = rootDirection

	nodes := make(map[string]*d2graph.Object, len(input.Nodes))
	for _, node := range input.Nodes {
		if _, exists := nodes[node.ID]; node.ID == "" || exists {
			return nil, nil, nil, fmt.Errorf("missing or duplicate node ID %q", node.ID)
		}
		if !measuredDimensions(node.Width, node.Height) {
			return nil, nil, nil, fmt.Errorf("node %q has invalid measured dimensions", node.ID)
		}
		nodeShape, err := shape(node.Shape)
		if node.IsGroup {
			nodeShape, err = d2target.ShapeRectangle, nil
		}
		if err != nil {
			return nil, nil, nil, fmt.Errorf("node %q: %w", node.ID, err)
		}
		nodeDirection, err := direction(node.Direction)
		if err != nil {
			return nil, nil, nil, fmt.Errorf("node %q: %w", node.ID, err)
		}
		attrs, err := node.attributes()
		if err != nil {
			return nil, nil, nil, fmt.Errorf("node %q: %w", node.ID, err)
		}
		attrs.Shape.Value = nodeShape
		attrs.Direction.Value = nodeDirection
		// An explicit font size tells TALA that these dimensions have already
		// been measured at a fixed size and should not be density-scaled.
		attrs.Style.FontSize = &d2graph.Scalar{Value: "16"}

		// D2 keys are case-insensitive and dot-separated. Encode the byte
		// sequence to preserve distinct Ferrigura IDs and stable D2 identities.
		key := "n" + hex.EncodeToString([]byte(node.ID))
		width, height := node.Width, node.Height
		if node.IsGroup {
			// Ferrigura measures the title before a group has a rectangle.
			// D2 requires a positive initial box and packs children afterward.
			width = max(1, width, float64(attrs.LabelDimensions.Width))
			height = max(1, height, float64(attrs.LabelDimensions.Height))
		}
		object := &d2graph.Object{
			Graph:      graph,
			ID:         key,
			IDVal:      key,
			Box:        geo.NewBox(nil, width, height),
			Children:   make(map[string]*d2graph.Object),
			Attributes: attrs,
		}
		nodes[node.ID] = object
		graph.Objects = append(graph.Objects, object)
	}
	for _, node := range input.Nodes {
		parent := graph.Root
		if node.ParentID != "" {
			parent = nodes[node.ParentID]
			if parent == nil {
				return nil, nil, nil, fmt.Errorf("unknown parent %q for node %q", node.ParentID, node.ID)
			}
		}
		object := nodes[node.ID]
		object.Parent = parent
		parent.Children[object.ID] = object
		parent.ChildrenArray = append(parent.ChildrenArray, object)
	}

	edges := make([]*d2graph.Edge, 0, len(input.Edges))
	seenEdges := make(map[string]bool, len(input.Edges))
	for index, edge := range input.Edges {
		if edge.ID == "" || seenEdges[edge.ID] {
			return nil, nil, nil, fmt.Errorf("missing or duplicate edge ID %q", edge.ID)
		}
		seenEdges[edge.ID] = true
		src, dst := nodes[edge.Source], nodes[edge.Target]
		if src == nil || dst == nil {
			return nil, nil, nil, fmt.Errorf("edge %q has an unknown endpoint", edge.ID)
		}
		attrs, err := edge.attributes()
		if err != nil {
			return nil, nil, nil, fmt.Errorf("edge %q: %w", edge.ID, err)
		}
		edges = append(edges, &d2graph.Edge{
			Index:      index,
			Src:        src,
			Dst:        dst,
			SrcArrow:   edge.SourceArrow,
			DstArrow:   edge.TargetArrow,
			Attributes: attrs,
		})
	}
	graph.Edges = append(graph.Edges, edges...)
	return graph, nodes, edges, nil
}

func pilotOptions() *d2talalayout.Options {
	return &d2talalayout.Options{Seeds: []int64{pilotSeed}, MaxConcurrency: 1}
}

func capture(ctx context.Context, input graphInput, options *d2talalayout.Options) (graphOutput, error) {
	graph, nodes, edges, err := makeGraph(input)
	if err != nil {
		return graphOutput{}, err
	}
	if err := d2talalayout.Layout(ctx, graph, options); err != nil {
		return graphOutput{}, err
	}

	output := graphOutput{
		Nodes: make([]nodeOutput, 0, len(nodes)),
		Edges: make([]edgeOutput, 0, len(edges)),
	}
	if options != nil && len(options.Seeds) == 1 {
		output.Seed = options.Seeds[0]
	}
	for _, node := range input.Nodes {
		object := nodes[node.ID]
		if object.TopLeft == nil {
			return graphOutput{}, fmt.Errorf("no result for node %q", node.ID)
		}
		output.Nodes = append(output.Nodes, nodeOutput{
			ID: node.ID, X: object.TopLeft.X, Y: object.TopLeft.Y,
			Width: object.Width, Height: object.Height,
		})
	}
	for index, edge := range edges {
		id := input.Edges[index].ID
		if edge.IsCurve || len(edge.Route) < 2 {
			return graphOutput{}, fmt.Errorf("edge %q has no polyline result", id)
		}
		result := edgeOutput{ID: id, Points: make([]point, 0, len(edge.Route))}
		for _, p := range edge.Route {
			result.Points = append(result.Points, point{X: p.X, Y: p.Y})
		}
		if edge.Label.Value != "" {
			if edge.LabelPosition == nil || edge.LabelPercentage == nil {
				return graphOutput{}, fmt.Errorf("no label placement for edge %q", id)
			}
			connection := d2target.BaseConnection()
			connection.Route = edge.Route
			connection.LabelPosition = *edge.LabelPosition
			connection.LabelPercentage = *edge.LabelPercentage
			connection.LabelWidth = edge.LabelDimensions.Width
			connection.LabelHeight = edge.LabelDimensions.Height
			origin := connection.GetLabelTopLeft()
			if origin == nil {
				return graphOutput{}, fmt.Errorf("cannot resolve label placement for edge %q", id)
			}
			result.Label = &point{
				X: origin.X + float64(connection.LabelWidth)/2,
				Y: origin.Y + float64(connection.LabelHeight)/2,
			}
			result.LabelWidth = connection.LabelWidth
			result.LabelHeight = connection.LabelHeight
		}
		output.Edges = append(output.Edges, result)
	}
	return output, nil
}

func readGraph(reader io.Reader) (graphInput, error) {
	decoder := json.NewDecoder(reader)
	decoder.DisallowUnknownFields()
	var input graphInput
	if err := decoder.Decode(&input); err != nil {
		return graphInput{}, fmt.Errorf("read measured flowchart: %w", err)
	}
	if err := decoder.Decode(new(any)); err != io.EOF {
		return graphInput{}, errors.New("expected one measured flowchart")
	}
	return input, nil
}

func benchmark(ctx context.Context, input graphInput, warmup, samples int) ([]int64, error) {
	times := make([]int64, 0, samples)
	options := pilotOptions()
	for i := -warmup; i < samples; i++ {
		graph, _, _, err := makeGraph(input)
		if err != nil {
			return nil, err
		}
		start := time.Now()
		if err := d2talalayout.Layout(ctx, graph, options); err != nil {
			return nil, err
		}
		elapsed := time.Since(start).Nanoseconds()
		if i >= 0 {
			times = append(times, elapsed)
		}
	}
	return times, nil
}

func run(ctx context.Context, reader io.Reader, writer io.Writer, args []string) error {
	input, err := readGraph(reader)
	if err != nil {
		return err
	}
	encoder := json.NewEncoder(writer)
	encoder.SetIndent("", "  ")
	switch {
	case len(args) == 0 || len(args) == 1 && args[0] == "--default":
		var options *d2talalayout.Options
		if len(args) == 0 {
			options = pilotOptions()
		}
		output, err := capture(ctx, input, options)
		if err != nil {
			return err
		}
		return encoder.Encode(output)
	case len(args) == 2:
		warmup, warmupErr := strconv.Atoi(args[0])
		samples, samplesErr := strconv.Atoi(args[1])
		if warmupErr != nil || warmup < 0 || samplesErr != nil || samples < 1 {
			return errors.New("benchmark requires nonnegative warmup and positive sample counts")
		}
		times, err := benchmark(ctx, input, warmup, samples)
		if err != nil {
			return err
		}
		return encoder.Encode(times)
	default:
		return errors.New("usage: tala-reference [--default | warmup samples]")
	}
}

func main() {
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	if err := run(ctx, os.Stdin, os.Stdout, os.Args[1:]); err != nil {
		fmt.Fprintln(os.Stderr, "tala reference:", err)
		os.Exit(1)
	}
}
