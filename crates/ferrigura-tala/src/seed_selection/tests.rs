// SPDX-License-Identifier: MPL-2.0
// Source: D2 b463ea5, d2talalayout/{adapter,result,seed_selection}_test.go.

use super::*;
use std::sync::Barrier;
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};

const fn score(penalty: f64, area: f64) -> LayoutScore {
    LayoutScore { penalty, area }
}

#[test]
fn score_compare() {
    use Ordering::{Equal, Greater, Less};
    for (left, right, expected) in [
        (score(1., 100.), score(2., 1.), Less),
        (score(2., 1.), score(1., 100.), Greater),
        (score(1., 10.), score(1., 20.), Less),
        (score(1., 20.), score(1., 10.), Greater),
        (score(1., 10.25), score(1., 10.5), Less),
        (score(1., 1e18), score(1., 1e18 + 256.), Less),
        (score(1., 10.), score(1., 10.), Equal),
        (score(1., 0.), score(f64::NAN, 0.), Less),
        (score(f64::NAN, 0.), score(1., 0.), Greater),
        (score(f64::INFINITY, 0.), score(f64::NAN, 0.), Equal),
        (score(1., 10.), score(1., f64::NAN), Less),
        (score(1., f64::NAN), score(1., 10.), Greater),
        (score(1., 10.), score(1., f64::INFINITY), Less),
        (score(1., 10.), score(1., -1.), Less),
        (score(1., f64::NEG_INFINITY), score(1., f64::NAN), Equal),
        (score(-0., 0.), score(0., -0.), Equal),
    ] {
        assert_eq!(left.compare(right), expected, "{left:?} vs {right:?}");
    }
}

#[test]
fn normalize_seeds_in_first_seen_order() {
    assert_eq!(normalize_seeds(&[3, 1, 3, 2, 1]), Ok(vec![3, 1, 2]));
    assert_eq!(normalize_seeds(&[]), Err(PlanError::EmptySeeds));

    let max = (0..16).collect::<Vec<_>>();
    assert_eq!(normalize_seeds(&max), Ok(max.clone()));
    assert_eq!(
        normalize_seeds(&max.iter().copied().chain([16]).collect::<Vec<_>>()),
        Err(PlanError::TooManySeeds)
    );
    assert_eq!(normalize_seeds(&max.repeat(2)), Ok(max));
    assert_eq!(normalize_seeds(&[0; 65]), Err(PlanError::TooManyEntries));

    assert_eq!(
        SeedPlan::new(Some(&[3, 2]), Some(&[1]), 16).unwrap(),
        SeedPlan {
            seeds: vec![3, 2],
            concurrency: 2,
        }
    );
    assert_eq!(SeedPlan::new(None, None, 1).unwrap().seeds(), &[1, 2, 3]);
    for invalid in [-1, 17] {
        assert_eq!(
            SeedPlan::new(None, None, invalid),
            Err(PlanError::InvalidConcurrency)
        );
    }
}

#[test]
fn seed_selection_across_completion_orders() {
    let orders = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    for (scores, expected) in [
        (
            [
                score(1. - 1. / 115., 300.),
                score(1. - 1. / 116., 200.),
                score(1. - 1. / 117., 100.),
            ],
            0,
        ),
        (
            [score(1.0_f64.next_up(), 1.), score(1., 300.), score(2., 1.)],
            1,
        ),
        ([score(1., 300.), score(1., 100.), score(1., 200.)], 1),
        ([score(1., 100.); 3], 2),
    ] {
        for order in orders {
            let mut selection = Selection::<_, ()>::new(3);
            for index in order {
                selection.consider(
                    index,
                    [1, 2, 3][index],
                    Ok(Scored {
                        score: scores[index],
                        candidate: index,
                    }),
                );
            }
            let selected = selection.finish(false).unwrap();
            assert_eq!(selected.result.candidate, expected, "{scores:?} {order:?}");
        }
    }
}

#[test]
fn all_attempts_run_with_bounded_concurrency() {
    let plan = SeedPlan::new(None, Some(&[1, 2, 3, 4, 5, 6]), 2).unwrap();
    let current = AtomicUsize::new(0);
    let maximum = AtomicUsize::new(0);
    let completed = AtomicUsize::new(0);
    let first_workers = Barrier::new(2);

    let winner = plan
        .coordinate(
            || false,
            |index, seed| {
                let active = current.fetch_add(1, SeqCst) + 1;
                maximum.fetch_max(active, SeqCst);
                if index < 2 {
                    first_workers.wait();
                }
                completed.fetch_add(1, SeqCst);
                current.fetch_sub(1, SeqCst);
                Ok::<_, ()>(Scored {
                    score: score(0., 0.),
                    candidate: seed,
                })
            },
        )
        .unwrap();

    assert_eq!(winner.result.candidate, 6);
    assert_eq!(completed.load(SeqCst), 6);
    assert_eq!(maximum.load(SeqCst), 2);
}

#[test]
fn tolerates_partial_failure() {
    let plan = SeedPlan::new(None, Some(&[6, 2, 9]), 3).unwrap();
    let winner = plan
        .coordinate(
            || false,
            |_, seed| {
                (seed == 2)
                    .then_some(Scored {
                        score: score(2., 0.),
                        candidate: seed,
                    })
                    .ok_or("failed")
            },
        )
        .unwrap();
    assert_eq!(winner.seed, 2);
}

#[test]
fn orders_all_failures_by_configured_seed() {
    let plan = SeedPlan::new(None, Some(&[6, 2, 9]), 3).unwrap();
    let failure = plan
        .coordinate::<(), _>(|| false, |_, seed| Err(seed.to_string()))
        .unwrap_err();
    assert_eq!(
        failure,
        SelectionError::AllFailed(
            [6, 2, 9]
                .into_iter()
                .map(|seed| SeedFailure {
                    seed,
                    error: AttemptError::Failed(seed.to_string()),
                })
                .collect()
        )
    );
}

#[test]
fn parallel_completion_does_not_change_selected_candidate() {
    let seeds = [1, 2, 3];
    let plan = SeedPlan::new(None, Some(&seeds), 3).unwrap();
    let started = Barrier::new(3);
    let winner = plan
        .coordinate(
            || false,
            |_, seed| {
                started.wait();
                Ok::<_, ()>(Scored {
                    score: score(1., 1.),
                    candidate: [seed; 4],
                })
            },
        )
        .unwrap();

    assert_eq!(winner.result.candidate, [3; 4]);
    assert_eq!(winner.index, 2);
}

#[test]
fn cancellation_joins_workers_before_rejecting_completed_candidate() {
    let plan = SeedPlan::new(None, Some(&[1, 2]), 2).unwrap();
    let started = Barrier::new(3);
    let resume = Barrier::new(2);
    let cancelled = AtomicBool::new(false);
    let exited = AtomicBool::new(false);

    thread::scope(|workers| {
        let call = workers.spawn(|| {
            plan.coordinate(
                || cancelled.load(SeqCst),
                |_, seed| {
                    started.wait();
                    if seed == 2 {
                        resume.wait();
                        exited.store(true, SeqCst);
                        return Err("cancelled");
                    }
                    Ok(Scored {
                        score: score(1., 0.),
                        candidate: seed,
                    })
                },
            )
        });
        started.wait();
        cancelled.store(true, SeqCst);
        resume.wait();

        assert_eq!(call.join().unwrap(), Err(SelectionError::Cancelled));
        assert!(exited.load(SeqCst));
    });
}
