/**
 * Configuration constants for the Swimlanes layout algorithm.
 * Centralizes all magic numbers and default values for maintainability.
 */

/**
 * Numerical precision constants
 */
export const PRECISION = {
  /** Epsilon for floating-point comparisons */
  EPSILON: 1e-6,
} as const;

/**
 * Layer assignment constants
 */
export const LAYERING = {
  /** Default number of iterations for gravity-based layering */
  GRAVITY_ITERATIONS: 8,

  /** Maximum number of passes for crossing-based rank optimization */
  MAX_CROSSING_OPTIMIZATION_PASSES: 4,

  /** Whether to compact single-input nodes by default */
  DEFAULT_COMPACT_SINGLE_INPUT: true,
} as const;

/**
 * Coordinate assignment constants
 */
export const COORDINATES = {
  /** Default vertical gap between layers (px) */
  DEFAULT_LAYER_GAP: 100,

  /** Default horizontal gap between nodes (px) */
  DEFAULT_NODE_GAP: 40,
} as const;
