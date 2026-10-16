/**
 * The flowchart fixtures in each stage of M5, and the layouts and looks that are out of scope.
 *
 * The lists come from `docs-port/flowchart/render-notes/fixtures-and-harness.md`, section 5. Paths
 * are relative to `e2e/diagrams/flowchart`. Each stage lists only its own files: `stage2` is the
 * 28 subgraph and edge-label files added on top of stage 1, 1a and 1b, so the set that stage 2
 * covers is `stage1,stage1a,stage1b,stage2`.
 */

export const STAGES: Readonly<Record<string, readonly string[]>> = {
  /** Classic shapes, no subgraphs, no edge labels, default appearance. */
  stage1: [
    'nightly-check/05-should-include-classes-on-the-edges.mmd',
    'nightly-check/06-should-render-a-flowchart-with-title.mmd',
    'nightly-check/07-render-with-stylized-arrows.mmd',
    'nightly-check/6088-5-should-handle-diamond-shape-intersections.mmd',
    'nightly-check/6647-should-keep-node-order-when-using-layout-unless-it-would-add-crossings.mmd',
    'pr-check/1-shapes-layout-default.mmd',
    'pr-check/4-classdef-style.mmd',
    'visual-check/1433-should-render-a-titled-flowchart-with-titletopmargin-set-to-0.mmd',
    'visual-check/3192-it-should-be-possible-to-render-flowcharts-with-invisible-edges.mmd',
    'visual-check/34-testing-the-label-width-in-percy.mmd',
    'visual-check/36-should-render-escaped-without-html-labels.mmd',
    'visual-check/5-should-render-escaped-without-html-labels.mmd',
    'visual-check/should-render-a-stadium-shaped-node.mmd',
  ],
  /** Shape aliases (`@{ shape: … }`) that map to classic shapes. */
  stage1a: [
    'nightly-check/shape-alias/01-all-process-rect-proc-rectangle-should-render-same-shape.mmd',
    'nightly-check/shape-alias/03-all-stadium-pill-terminal-should-render-same-shape.mmd',
    'nightly-check/shape-alias/04-all-fr-rect-subproc-subprocess-framed-rectangle-subroutine-should-render-same-shape.mmd',
    'nightly-check/shape-alias/05-all-db-database-cylinder-cyl-should-render-same-shape.mmd',
    'nightly-check/shape-alias/06-all-diam-decision-diamond-should-render-same-shape.mmd',
    'nightly-check/shape-alias/07-all-hex-hexagon-prepare-should-render-same-shape.mmd',
    'nightly-check/shape-alias/12-all-dbl-circ-double-circle-should-render-same-shape.mmd',
  ],
  /** Stage 1 criteria with `fontFamily: courier`. Enters after the font decision (open question 1). */
  stage1b: [
    'pr-check/12-should-render-a-flowchart-with-long-names-and-class-definitions.mmd',
    'visual-check/13-should-render-color-of-styled-nodes.mmd',
    'visual-check/18-chaining-of-nodes.mmd',
    'visual-check/19-multiple-nodes-and-chaining-in-one-statement.mmd',
    'visual-check/25-handle-link-click-events-link-anchor-mailto-other-protocol-script.mmd',
    'visual-check/29-apply-default-class-to-all-nodes-which-do-not-have-another-class-assigned-htmllabels-disabled.mmd',
    'visual-check/31-should-not-slice-off-edges-that-are-to-the-left-of-the-left-most-vertex.mmd',
    'visual-check/5-should-style-nodes-via-a-class.mmd',
    'visual-check/6-should-render-a-flowchart-full-of-circles.mmd',
    'visual-check/8-should-render-labels-with-numbers-at-the-start.mmd',
    'visual-check/stadium-tall-labels.mmd',
  ],
  /** Subgraphs of depth 1 and 2 (no subgraph direction) and edge labels. New files only. */
  stage2: [
    'nightly-check/3-a-link-with-correct-arrowhead-to-a-subgraph.mmd',
    'nightly-check/4-length-of-edges.mmd',
    'nightly-check/6088-6-should-handle-diamond-shape-intersections.mmd',
    'nightly-check/7213-should-render-edges-with-right-angles-not-curves.mmd',
    'nightly-check/8292-elk-edge-label-jog.mmd',
    'nightly-check/8326-should-merge-repeated-subgraph-id-with-nested-subgraph.mmd',
    'nightly-check/8326-should-merge-repeated-subgraph-ids.mmd',
    'nightly-check/recursive-flow-with-keepentrynodeontop-false-keeps-the-default-layout-7827.mmd',
    'nightly-check/redux-color-subgraphs/05-should-render-subgraphs-subgraph-containers.mmd',
    'nightly-check/redux-color-subgraphs/06-should-render-nested-subgraph-containers.mmd',
    'nightly-check/redux-color-subgraphs/08-should-render-user-styled-subgraph-containers.mmd',
    'nightly-check/should-keep-the-entry-node-on-top-of-a-recursive-flow-nested-in-a-subgraph-with-keepentrynodeontop-true-7827.mmd',
    'nightly-check/should-keep-the-entry-node-on-top-when-a-flow-recurses-with-keepentrynodeontop-true-7827.mmd',
    'nightly-check/should-merge-edges-within-subgraphs-when-mergeedges-is-true-7659.mmd',
    'nightly-check/sub-graphs.mmd',
    'pr-check/3-layout-elk-subgraphs.mmd',
    'pr-check/8368-elk-label-parallel-edge.mmd',
    'visual-check/3-a-link-with-correct-arrowhead-to-a-subgraph.mmd',
    'visual-check/35-should-honor-minimum-edge-length-as-specified-by-the-user.mmd',
    'visual-check/4-length-of-edges.mmd',
    'visual-check/4b-labeled-open-edges-should-not-have-arrowheads.mmd',
    'visual-check/5059-should-render-when-subgraph-contains-only-subgraphs-has-link-to-outside-and-itself-is-part-of-a-link.mmd',
    'visual-check/5064-should-render-when-subgraph-child-has-links-to-outside-node-and-subgraph.mmd',
    'visual-check/5a-should-render-flowchart-with-edge-labels-centered-when-htmllabels-is-false.mmd',
    'visual-check/7213-should-render-edges-with-rounded-curve-right-angles-with-rounded-corners.mmd',
    'visual-check/should-keep-the-true-entry-on-top-when-a-back-edge-feeds-it-79.mmd',
    'visual-check/should-render-labeled-self-loops.mmd',
    'visual-check/subgraph-titles-wider-than-children.mmd',
  ],
};

/**
 * Layouts that are not layered. ferrigura implements `elk.layered` only and falls back to it for
 * the others, so a fixture that names one of these is excluded from the comparison.
 */
export const NON_LAYERED_LAYOUTS: readonly string[] = [
  'elk.box',
  'elk.rectpacking',
  'elk.stress',
  'elk.force',
  'elk.mrtree',
  'elk.sporeOverlap',
  'cose-bilkent',
  'swimlane',
];

/** The look that is out of scope: rough.js sketching is not part of the port. */
export const EXCLUDED_LOOK = 'handDrawn';
