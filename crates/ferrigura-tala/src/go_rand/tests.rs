// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::convert::Infallible;

use serde::Deserialize;

use super::{GoRand, INT63_RANGE, STATE_LENGTH, TAP_DISTANCE};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Vectors {
    go_version: String,
    lengths: Vec<usize>,
    wide_bounds: Vec<u64>,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Case {
    seed: i64,
    shuffles: Vec<Shuffle>,
    float_bits: Vec<String>,
    wide_indices: Vec<u64>,
}

#[derive(Deserialize)]
struct Shuffle {
    draws: u64,
    digest: String,
}

#[test]
fn mixed_shuffles_and_jitter_match_pinned_go_bit_for_bit() {
    // Traced using Go's rand.New(rand.NewSource(seed)), TALA's actual
    // limits.Shuffle including guard.Used(), and rand.Float64 between calls.
    let vectors: Vectors = serde_json::from_str(include_str!("vectors.json")).unwrap();
    assert_eq!(vectors.go_version, "go1.27.4");
    for case in vectors.cases {
        let mut rng = GoRand::new(case.seed);
        let mut expected_floats = case.float_bits.iter();
        assert_eq!(case.shuffles.len(), vectors.lengths.len());
        for (&length, expected) in vectors.lengths.iter().zip(case.shuffles) {
            let mut values: Vec<_> = (0..u64::try_from(length).unwrap()).collect();
            assert_next_float(&mut rng, expected_floats.next().unwrap());
            let mut draws = 0;
            rng.shuffle(&mut values, || {
                draws += 1;
                Ok::<_, Infallible>(())
            })
            .unwrap();
            assert_eq!(draws, expected.draws, "seed {}, length {length}", case.seed);
            assert_eq!(
                fingerprint(&values),
                expected.digest,
                "seed {}, length {length}",
                case.seed
            );
            assert_next_float(&mut rng, expected_floats.next().unwrap());
        }
        let actual: Vec<_> = vectors
            .wide_bounds
            .iter()
            .map(|&upper| rng.int63_below(upper))
            .collect();
        assert_eq!(actual, case.wide_indices, "seed {}", case.seed);
        assert_next_float(&mut rng, expected_floats.next().unwrap());
        assert!(expected_floats.next().is_none());
    }
}

#[test]
fn shuffle_charges_rejected_draws_before_moving_items() {
    let mut random = source_with_initial_draws(&[0, u64::from(u32::MAX) << 31, 0]);
    let mut values = [0, 1, 2];
    let mut draws = 0;
    random
        .shuffle(&mut values, || {
            draws += 1;
            Ok::<_, Infallible>(())
        })
        .unwrap();
    assert_eq!(values, [1, 0, 2]);
    assert_eq!(draws, 3);

    let mut random = source_with_initial_draws(&[0, u64::from(u32::MAX) << 31]);
    let mut values = [0, 1, 2];
    let mut remaining = 1;
    assert_eq!(
        random.shuffle(&mut values, || {
            if remaining == 0 {
                return Err(());
            }
            remaining -= 1;
            Ok(())
        }),
        Err(())
    );
    assert_eq!(values, [0, 1, 2]);
    assert_eq!(random.uint32(), u32::MAX);
}

#[test]
fn float64_retries_a_draw_that_rounds_to_one() {
    let mut rng = source_with_initial_draws(&[INT63_RANGE - 1, INT63_RANGE / 2, 27]);
    assert_eq!(rng.float64(), 0.5);
    assert_eq!(rng.int63(), 27);
}

fn assert_next_float(rng: &mut GoRand, expected: &str) {
    assert_eq!(format!("{:016x}", rng.float64().to_bits()), expected);
}

fn fingerprint(values: &[u64]) -> String {
    let mut state = 0xcbf2_9ce4_8422_2325_u64;
    for byte in values.iter().flat_map(|value| value.to_le_bytes()) {
        state = (state ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{state:016x}")
}

fn source_with_initial_draws(draws: &[u64]) -> GoRand {
    let mut state = [0; STATE_LENGTH];
    for (word, &draw) in state.iter_mut().rev().zip(draws) {
        *word = draw;
    }
    GoRand {
        state,
        tap: 0,
        feed: STATE_LENGTH - TAP_DISTANCE,
    }
}
