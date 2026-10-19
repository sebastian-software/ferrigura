//! The boundary waves and the seed of the cynefin renderer, as `packages/mermaid/src/diagrams/cynefin/
//! cynefin.spec.ts` tests them (the boundary and seed blocks, lines 105-191), with the vectors that
//! the probes of `docs-port/cynefin/NOTES.md` section 6.5 recorded in Chromium's JavaScript. The
//! parser's and the DB's specs are in `ferrigura-syntax` (`cynefin_db_spec.rs`, `cynefin_grammar.rs`).

use ferrigura_render::cynefin::boundary::{
    cliff_path, confusion_path, fold_path, hash_string, horizontal_path, resolve_seed,
    seeded_random,
};

/// `seededRandom` is deterministic and in [0, 1) for the seeds of the reference's spec.
#[test]
fn seeded_random_should_return_deterministic_values() {
    assert_eq!(seeded_random(42.0), seeded_random(42.0));
}

#[test]
fn seeded_random_should_return_values_between_0_and_1() {
    for seed in [0.0, 1.0, 100.0, 999_999.0, -50.0] {
        let value = seeded_random(seed);
        assert!((0.0..1.0).contains(&value), "seed {seed} gave {value}");
    }
}

/// `| 0` truncates a fractional seed toward zero, so 1 and 1.5 draw the same waves (probed).
#[test]
fn seeded_random_truncates_a_fractional_seed() {
    assert_eq!(seeded_random(1.0), seeded_random(1.5));
    assert_eq!(
        fold_path(800.0, 600.0, 1.0, 8.0),
        fold_path(800.0, 600.0, 1.5, 8.0)
    );
}

#[test]
fn hash_string_should_return_consistent_hashes() {
    assert_eq!(hash_string("cynefin"), hash_string("cynefin"));
}

#[test]
fn hash_string_should_return_different_hashes_for_different_strings() {
    assert_ne!(hash_string("complex"), hash_string("chaotic"));
}

/// The hash of the probed id, in 32-bit arithmetic (`hashString('ref-seed-a')`, probed).
#[test]
fn hash_string_matches_the_probed_value() {
    assert_eq!(hash_string("ref-seed-a"), -1_042_255_233);
}

#[test]
fn fold_path_should_return_valid_svg_path() {
    let path = fold_path(800.0, 600.0, 42.0, 8.0);
    assert!(path.starts_with('M'), "{path}");
    assert!(path.contains('C'), "{path}");
}

#[test]
fn fold_path_should_be_deterministic() {
    assert_eq!(
        fold_path(800.0, 600.0, 42.0, 8.0),
        fold_path(800.0, 600.0, 42.0, 8.0)
    );
}

/// The fold of seed 42 and amplitude 8 begins as the probes recorded (NOTES 6.5, the test vector).
#[test]
fn fold_path_matches_the_probed_vector_for_seed_42() {
    let path = fold_path(800.0, 600.0, 42.0, 8.0);
    assert!(
        path.starts_with(
            "M401.6176600307226,0 C406.1051963418722,42.857142857142854 393.83792743459344,42.857142857142854 398.32546374574304,85.71428571428571"
        ),
        "{path}"
    );
}

#[test]
fn fold_path_matches_the_probed_vector_for_seed_1() {
    let path = fold_path(800.0, 600.0, 1.0, 8.0);
    assert!(
        path.starts_with(
            "M402.0331830494106,0 C403.9087250875309,42.857142857142854 396.49504977371544,42.857142857142854 398.37059181183577,85.71428571428571"
        ),
        "{path}"
    );
}

/// Amplitude 0 gives straight boundaries (probed): the `??` keeps the 0, and every point is on the
/// centre line, so the fold stays at x = 400.
#[test]
fn an_amplitude_of_zero_draws_a_straight_fold() {
    let path = fold_path(800.0, 600.0, 42.0, 0.0);
    for point in path.split(['M', 'C', ' ']).filter(|part| !part.is_empty()) {
        let x: f64 = point
            .split(',')
            .next()
            .unwrap_or("")
            .parse()
            .unwrap_or(f64::NAN);
        assert_eq!(x, 400.0, "{path}");
    }
}

#[test]
fn horizontal_boundary_should_return_valid_svg_path() {
    let path = horizontal_path(800.0, 600.0, 42.0, 8.0);
    assert!(path.starts_with('M'), "{path}");
    assert!(path.contains('C'), "{path}");
}

/// The cliff has a fixed amplitude and no seed, so its path is the same for every render.
#[test]
fn cliff_path_matches_the_probed_path() {
    assert_eq!(
        cliff_path(800.0, 600.0),
        "M400,300 C424,360 364,465 412,525 C376,555 407.2,585 400,600"
    );
}

#[test]
fn confusion_path_should_return_valid_ellipse_path() {
    let path = confusion_path(400.0, 300.0, 50.0, 40.0);
    assert!(path.starts_with('M'), "{path}");
    assert!(path.contains('A'), "{path}");
    assert!(path.ends_with('Z'), "{path}");
}

#[test]
fn confusion_path_should_use_provided_center_and_radii() {
    assert_eq!(
        confusion_path(400.0, 300.0, 50.0, 40.0),
        "M350,300 A50,40 0 1,1 450,300 A50,40 0 1,1 350,300 Z"
    );
}

/// `resolveSeed`: a non-zero configured seed is used as it is, and the others take the hash of the id.
#[test]
fn resolve_seed_returns_the_configured_seed_when_it_is_a_non_zero_number() {
    assert_eq!(resolve_seed(42.0, "mermaid-1"), 42.0);
    assert_eq!(resolve_seed(-3.0, "mermaid-2"), -3.0);
}

#[test]
fn resolve_seed_falls_back_to_hash_string_of_the_id_when_the_seed_is_zero() {
    assert_eq!(
        resolve_seed(0.0, "mermaid-1"),
        f64::from(hash_string("mermaid-1"))
    );
}

#[test]
fn resolve_seed_ignores_the_id_when_an_explicit_seed_is_configured() {
    assert_eq!(resolve_seed(7.0, "a"), resolve_seed(7.0, "b"));
}

#[test]
fn resolve_seed_produces_different_fallback_seeds_for_different_ids() {
    assert_ne!(
        resolve_seed(0.0, "mermaid-1"),
        resolve_seed(0.0, "mermaid-2")
    );
}

/// `NaN` and the infinities are not configured seeds (`Number.isFinite`), so they take the hash.
#[test]
fn resolve_seed_treats_nan_and_infinity_as_unconfigured() {
    let hashed = f64::from(hash_string("mermaid-1"));
    assert_eq!(resolve_seed(f64::NAN, "mermaid-1"), hashed);
    assert_eq!(resolve_seed(f64::INFINITY, "mermaid-1"), hashed);
}
