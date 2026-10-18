//! The boundary waves and the seed of the cynefin renderer, as `cynefin.spec.ts` tests them
//! (`cynefinBoundaries.ts`, and `resolveSeed`). The waves are drawn by the renderer, which is not
//! ported yet, so these tests stay here, ignored, with the reason. They name what the renderer
//! stage must reproduce: the seeded generator with its draws, the hash of the SVG id, and the
//! paths of the fold, the horizontal boundary, the cliff and the confusion ellipse.

#[test]
#[ignore = "port: the renderer stage draws the boundary waves (cynefinBoundaries.ts)"]
fn seeded_random_should_return_deterministic_values() {}

#[test]
#[ignore = "port: the renderer stage draws the boundary waves (cynefinBoundaries.ts)"]
fn seeded_random_should_return_values_between_0_and_1() {}

#[test]
#[ignore = "port: the renderer stage draws the boundary waves (cynefinBoundaries.ts)"]
fn hash_string_should_return_consistent_hashes() {}

#[test]
#[ignore = "port: the renderer stage draws the boundary waves (cynefinBoundaries.ts)"]
fn hash_string_should_return_different_hashes_for_different_strings() {}

#[test]
#[ignore = "port: the renderer stage draws the boundary waves (cynefinBoundaries.ts)"]
fn generate_fold_path_should_return_valid_svg_path() {}

#[test]
#[ignore = "port: the renderer stage draws the boundary waves (cynefinBoundaries.ts)"]
fn generate_fold_path_should_be_deterministic() {}

#[test]
#[ignore = "port: the renderer stage draws the boundary waves (cynefinBoundaries.ts)"]
fn generate_horizontal_boundary_should_return_valid_svg_path() {}

#[test]
#[ignore = "port: the renderer stage draws the boundary waves (cynefinBoundaries.ts)"]
fn generate_cliff_path_should_return_valid_svg_path() {}

#[test]
#[ignore = "port: the renderer stage draws the boundary waves (cynefinBoundaries.ts)"]
fn generate_confusion_path_should_return_valid_ellipse_path() {}

#[test]
#[ignore = "port: the renderer stage draws the boundary waves (cynefinBoundaries.ts)"]
fn generate_confusion_path_should_use_provided_center_and_radii() {}

#[test]
#[ignore = "port: the seed is resolved by the renderer (resolveSeed, cynefinBoundaries.ts)"]
fn resolve_seed_returns_the_configured_seed_when_it_is_a_non_zero_number() {}

#[test]
#[ignore = "port: the seed is resolved by the renderer (resolveSeed, cynefinBoundaries.ts)"]
fn resolve_seed_falls_back_to_hash_string_of_the_id_when_the_seed_is_zero() {}

#[test]
#[ignore = "port: the seed is resolved by the renderer (resolveSeed, cynefinBoundaries.ts)"]
fn resolve_seed_falls_back_to_hash_string_of_the_id_when_the_seed_is_undefined() {}

#[test]
#[ignore = "port: the seed is resolved by the renderer (resolveSeed, cynefinBoundaries.ts)"]
fn resolve_seed_ignores_the_id_when_an_explicit_seed_is_configured() {}

#[test]
#[ignore = "port: the seed is resolved by the renderer (resolveSeed, cynefinBoundaries.ts)"]
fn resolve_seed_produces_different_fallback_seeds_for_different_ids() {}

#[test]
#[ignore = "port: the seed is resolved by the renderer (resolveSeed, cynefinBoundaries.ts)"]
fn resolve_seed_treats_nan_and_infinity_as_unconfigured() {}
