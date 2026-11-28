package main

import (
	"bytes"
	"context"
	"encoding/json"
	"strings"
	"testing"
)

func TestCaptureIsRepeatable(t *testing.T) {
	const diagram = `{
  "direction": "LR",
  "nodes": [
    {"id":"one","isGroup":true,"width":50,"height":25,"label":"One","labelWidth":27,"labelHeight":17},
    {"id":"two","isGroup":true,"width":52,"height":25,"label":"Two","labelWidth":29,"labelHeight":17},
    {"id":"A","parentId":"one","width":100,"height":40},
    {"id":"a","parentId":"one","width":105,"height":45},
    {"id":"C.dot","parentId":"two","width":110,"height":52}
  ],
  "edges": [
    {"id":"first","source":"A","target":"a","targetArrow":true},
    {"id":"cross","source":"a","target":"C.dot","targetArrow":true,"label":"Across",
     "labelWidth":47.3,"labelHeight":17.2},
    {"id":"return","source":"C.dot","target":"A","targetArrow":true}
  ]
}`
	var first, second bytes.Buffer
	for _, buf := range []*bytes.Buffer{&first, &second} {
		if err := run(context.Background(), strings.NewReader(diagram), buf); err != nil {
			t.Fatal(err)
		}
	}
	if !bytes.Equal(first.Bytes(), second.Bytes()) {
		t.Fatal("the fixed seed produced different geometry")
	}

	var output graphOutput
	if err := json.Unmarshal(first.Bytes(), &output); err != nil {
		t.Fatal(err)
	}
	if output.Seed != pilotSeed || len(output.Nodes) != 5 || len(output.Edges) != 3 {
		t.Fatalf("unexpected result shape: %+v", output)
	}
	if output.Nodes[2].ID != "A" || output.Nodes[3].ID != "a" || output.Nodes[4].ID != "C.dot" {
		t.Fatalf("node identities moved: %+v", output.Nodes)
	}

	edge := output.Edges[1]
	if edge.ID != "cross" || len(edge.Points) < 2 || edge.Label == nil ||
		edge.LabelWidth != 48 || edge.LabelHeight != 18 {
		t.Fatalf("unexpected cross-group edge: %+v", edge)
	}
	for i := 1; i < len(edge.Points); i++ {
		a, b := edge.Points[i-1], edge.Points[i]
		if a == b || (a.X != b.X && a.Y != b.Y) {
			t.Fatalf("cross-group edge has a non-orthogonal or zero-length segment: %v to %v", a, b)
		}
	}
}

func TestUnknownEndpointReturnsNoGeometry(t *testing.T) {
	var output bytes.Buffer
	err := run(context.Background(), strings.NewReader(
		`{"nodes":[{"id":"a","width":10,"height":10}],"edges":[{"id":"broken","source":"a","target":"b"}]}`,
	), &output)
	if err == nil || !strings.Contains(err.Error(), "unknown endpoint") || output.Len() != 0 {
		t.Fatalf("expected an endpoint error and no result, got %q and %v", output.String(), err)
	}
}
