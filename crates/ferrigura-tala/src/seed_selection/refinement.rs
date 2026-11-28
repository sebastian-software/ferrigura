// SPDX-License-Identifier: MPL-2.0
// Source: D2 b463ea5, d2talalayout/{layout,candidate_test,refinement_test}.go.

use std::panic::{AssertUnwindSafe, catch_unwind};

use super::Scored;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interruption {
    Cancelled,
    DeadlineExceeded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrialFailure {
    Rejected,
    Interrupted(Interruption),
}

#[derive(Debug, PartialEq)]
pub struct TrialOutcome<T> {
    pub selected: Scored<T>,
    /// The caller must reject the selection when an interruption is present.
    pub interruption: Option<Interruption>,
}

/// Considers a speculative placement only when it strictly improves the score.
/// A new placement does not displace the incumbent when the caller interrupts
/// the attempt.
pub fn consider_placement_candidate<T>(
    incumbent: Scored<T>,
    interruption: impl FnOnce() -> Option<Interruption>,
    build: impl FnOnce(&Scored<T>) -> Result<Scored<T>, TrialFailure>,
) -> TrialOutcome<T> {
    let (candidate, local_interruption) = try_candidate(&incumbent, build);
    let interruption = interruption().or(local_interruption);
    let selected = if interruption.is_none() {
        candidate
            .filter(|candidate| candidate.score.compare(incumbent.score).is_lt())
            .unwrap_or(incumbent)
    } else {
        incumbent
    };
    TrialOutcome {
        selected,
        interruption,
    }
}

/// Retains a successful routing-only refinement even if its generic score is
/// worse. The caller validates that the refinement preserves its intended
/// placement. A rejected or panicking trial leaves the whole incumbent intact.
pub fn refine_selected_routes<T>(
    incumbent: Scored<T>,
    interruption: impl FnOnce() -> Option<Interruption>,
    build: impl FnOnce(&Scored<T>) -> Result<Scored<T>, TrialFailure>,
) -> TrialOutcome<T> {
    let (candidate, local_interruption) = try_candidate(&incumbent, build);
    TrialOutcome {
        selected: candidate.unwrap_or(incumbent),
        interruption: interruption().or(local_interruption),
    }
}

fn try_candidate<T>(
    incumbent: &Scored<T>,
    build: impl FnOnce(&Scored<T>) -> Result<Scored<T>, TrialFailure>,
) -> (Option<Scored<T>>, Option<Interruption>) {
    match catch_unwind(AssertUnwindSafe(|| build(incumbent))) {
        Ok(Ok(candidate)) => (Some(candidate), None),
        Ok(Err(TrialFailure::Interrupted(error))) => (None, Some(error)),
        Ok(Err(TrialFailure::Rejected)) | Err(_) => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed_selection::LayoutScore;
    use std::cell::Cell;

    fn candidate<T>(payload: T, penalty: f64, area: f64) -> Scored<T> {
        Scored {
            score: LayoutScore { penalty, area },
            candidate: payload,
        }
    }

    #[test]
    fn optional_seed_candidate_preserves_valid_incumbent() {
        for (trial, expected) in [(0, 1), (1, 1), (2, 1), (3, 1), (4, 2)] {
            let result = consider_placement_candidate(
                candidate(1, 10.0, 0.0),
                || None,
                |_| match trial {
                    0 => Err(TrialFailure::Rejected),
                    1 => panic!("optional invariant"),
                    2 => Ok(candidate(2, 11.0, 0.0)),
                    3 => Ok(candidate(2, 10.0, 0.0)),
                    4 => Ok(candidate(2, 9.0, 0.0)),
                    _ => unreachable!(),
                },
            );
            assert_eq!(result.selected.candidate, expected);
            assert_eq!(result.interruption, None);
        }
    }

    #[test]
    fn optional_seed_candidate_cancellation_propagates() {
        let cancelled = Cell::new(false);
        let result = consider_placement_candidate(
            candidate(1, 10.0, 0.0),
            || cancelled.get().then_some(Interruption::Cancelled),
            |_| {
                cancelled.set(true);
                panic!("interrupted proposal");
            },
        );
        assert_eq!(result.interruption, Some(Interruption::Cancelled));
    }

    #[test]
    fn optional_seed_refinement_keeps_selected_placement_despite_higher_score() {
        let bodies = [(0, 0, 40, 40), (200, 0, 40, 40)];
        let original = (bodies, vec![(40, 20), (200, 20)], Vec::<i64>::new());
        let routed = (
            bodies,
            vec![
                (40, 20),
                (80, 20),
                (80, 80),
                (160, 80),
                (160, 20),
                (200, 20),
            ],
            vec![3],
        );
        let scored_route = || candidate(routed.clone(), 11.0, 19_200.0);
        let incumbent = || candidate(original.clone(), 10.0, 9_600.0);
        assert!(scored_route().score.compare(incumbent().score).is_gt());

        let refinement = refine_selected_routes(incumbent(), || None, |_| Ok(scored_route()));
        assert_eq!(refinement.selected, scored_route());
        assert_eq!(refinement.selected.candidate.0, original.0);
        assert_eq!(refinement.interruption, None);

        let ordinary = consider_placement_candidate(incumbent(), || None, |_| Ok(scored_route()));
        assert_eq!(ordinary.selected, incumbent());
        assert_eq!(ordinary.interruption, None);
    }

    #[test]
    fn optional_seed_refinement_failure_preserves_complete_selected_result() {
        let incumbent = || candidate(vec![7], 10.0, 100.0);
        for (trial, expected) in [
            (0, None),
            (1, None),
            (2, Some(Interruption::Cancelled)),
            (3, Some(Interruption::DeadlineExceeded)),
        ] {
            let result = refine_selected_routes(
                incumbent(),
                || None,
                |_| match trial {
                    0 => Err(TrialFailure::Rejected),
                    1 => panic!("optional routing invariant"),
                    _ => Err(TrialFailure::Interrupted(expected.unwrap())),
                },
            );
            assert_eq!(result.selected, incumbent());
            assert_eq!(result.interruption, expected);
        }
    }

    #[test]
    fn optional_seed_refinement_caller_cancellation_always_propagates() {
        for trial in 0..3 {
            let cancelled = Cell::new(false);
            let result = refine_selected_routes(
                candidate(1, 10.0, 0.0),
                || cancelled.get().then_some(Interruption::Cancelled),
                |_| {
                    cancelled.set(true);
                    match trial {
                        0 => Ok(candidate(2, 0.0, 0.0)),
                        1 => Err(TrialFailure::Rejected),
                        _ => panic!("interrupted refinement"),
                    }
                },
            );
            assert_eq!(result.interruption, Some(Interruption::Cancelled));
        }
    }
}
