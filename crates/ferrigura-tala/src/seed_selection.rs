// SPDX-License-Identifier: MPL-2.0
// Source: D2 b463ea5, d2talalayout/{layout,result,seeds_sched,seeds_nosched}.go.

use std::cmp::Ordering;
use std::fmt;
use std::num::NonZeroUsize;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::sync::mpsc::sync_channel;
use std::thread;

use serde::{Deserialize, Serialize};

const DEFAULT_SEEDS: [i64; 3] = [1, 2, 3];
const MAX_SEEDS: usize = 16;
const MAX_SEED_ENTRIES: usize = MAX_SEEDS * 4;
const DEFAULT_MAX_CONCURRENCY: usize = 4;

/// Penalty and exact area tie-breaker for a completed layout.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct LayoutScore {
    pub penalty: f64,
    pub area: f64,
}

impl LayoutScore {
    pub fn compare(self, other: Self) -> Ordering {
        compare_number(self.penalty, other.penalty, false)
            .then_with(|| compare_number(self.area, other.area, true))
    }
}

fn compare_number(left: f64, right: f64, nonnegative: bool) -> Ordering {
    let valid = |value: f64| value.is_finite() && (!nonnegative || value >= 0.0);
    match (valid(left), valid(right)) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => Ordering::Equal,
        (true, true) => {
            if left < right {
                Ordering::Less
            } else if left > right {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        }
    }
}

/// Resolved seed order and upper bound for simultaneous complete attempts.
#[derive(Debug, PartialEq, Eq)]
pub struct SeedPlan {
    seeds: Vec<i64>,
    concurrency: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanError {
    EmptySeeds,
    TooManyEntries,
    TooManySeeds,
    InvalidConcurrency,
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySeeds => f.write_str("tala requires at least one seed"),
            Self::TooManyEntries => {
                write!(f, "tala accepts at most {MAX_SEED_ENTRIES} seed entries")
            }
            Self::TooManySeeds => write!(f, "tala supports at most {MAX_SEEDS} unique seeds"),
            Self::InvalidConcurrency => write!(
                f,
                "tala MaxConcurrency must be between 1 and {MAX_SEEDS}, or zero for the default"
            ),
        }
    }
}

impl std::error::Error for PlanError {}

impl SeedPlan {
    /// Preconfigured graph data takes precedence over caller options if present.
    ///
    /// # Errors
    /// Rejects an empty seed list, excessive entries or distinct seeds, and
    /// concurrency outside the allowed range.
    pub fn new(
        graph_seeds: Option<&[i64]>,
        seeds: Option<&[i64]>,
        concurrency: i32,
    ) -> Result<Self, PlanError> {
        let seeds = normalize_seeds(graph_seeds.or(seeds).unwrap_or(&DEFAULT_SEEDS))?;
        let concurrency =
            usize::try_from(concurrency).map_err(|_| PlanError::InvalidConcurrency)?;
        let concurrency = match concurrency {
            0 => thread::available_parallelism()
                .map_or(1, NonZeroUsize::get)
                .min(DEFAULT_MAX_CONCURRENCY),
            1..=MAX_SEEDS => concurrency,
            _ => return Err(PlanError::InvalidConcurrency),
        };
        Ok(Self {
            concurrency: concurrency.min(seeds.len()),
            seeds,
        })
    }

    pub fn seeds(&self) -> &[i64] {
        &self.seeds
    }

    pub fn concurrency(&self) -> usize {
        self.concurrency
    }

    /// Runs all indexed seed attempts before selecting or rejecting a result.
    ///
    /// Lower scores win. On an exact score tie the later configured seed wins,
    /// independently of worker completion order. The selected payload is
    /// transferred unchanged.
    ///
    /// # Errors
    /// Returns cancellation after all workers finish or the failures from every
    /// attempted seed in configured order when none produced a result.
    pub fn coordinate<T: Send, E: Send>(
        &self,
        cancelled: impl Fn() -> bool + Sync,
        run: impl Fn(usize, i64) -> Result<Scored<T>, E> + Sync,
    ) -> Result<Selected<T>, SelectionError<E>> {
        let mut selection = Selection::new(self.seeds.len());
        let attempt = |index: usize| {
            let seed = self.seeds[index];
            let result = match catch_unwind(AssertUnwindSafe(|| run(index, seed))) {
                Ok(result) => result.map_err(AttemptError::Failed),
                Err(_) => Err(AttemptError::Panicked),
            };
            (index, seed, result)
        };
        if self.concurrency == 1 || cfg!(target_family = "wasm") {
            for index in 0..self.seeds.len() {
                let (index, seed, result) = attempt(index);
                selection.consider(index, seed, result);
            }
        } else {
            let next = AtomicUsize::new(0);
            let (send, receive) = sync_channel(0);
            thread::scope(|workers| {
                for _ in 0..self.concurrency {
                    let send = send.clone();
                    let next = &next;
                    let attempt = &attempt;
                    workers.spawn(move || {
                        loop {
                            let index = next.fetch_add(1, Relaxed);
                            if index >= self.seeds.len() {
                                break;
                            }
                            if send.send(attempt(index)).is_err() {
                                break;
                            }
                        }
                    });
                }
                drop(send);
                for (index, seed, result) in receive {
                    selection.consider(index, seed, result);
                }
            });
        }
        selection.finish(cancelled())
    }
}

fn normalize_seeds(seeds: &[i64]) -> Result<Vec<i64>, PlanError> {
    if seeds.is_empty() {
        return Err(PlanError::EmptySeeds);
    }
    if seeds.len() > MAX_SEED_ENTRIES {
        return Err(PlanError::TooManyEntries);
    }
    let mut unique = Vec::with_capacity(seeds.len().min(MAX_SEEDS));
    for &seed in seeds {
        if !unique.contains(&seed) {
            unique.push(seed);
            if unique.len() > MAX_SEEDS {
                return Err(PlanError::TooManySeeds);
            }
        }
    }
    Ok(unique)
}

#[derive(Debug, PartialEq)]
pub struct Scored<T> {
    pub score: LayoutScore,
    pub candidate: T,
}

#[derive(Debug, PartialEq)]
pub struct Selected<T> {
    pub index: usize,
    pub seed: i64,
    pub result: Scored<T>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AttemptError<E> {
    Failed(E),
    Panicked,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SeedFailure<E> {
    pub seed: i64,
    pub error: AttemptError<E>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SelectionError<E> {
    Cancelled,
    AllFailed(Vec<SeedFailure<E>>),
}

impl<E: fmt::Display> fmt::Display for SelectionError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => f.write_str("tala layout stopped before all seeds completed"),
            Self::AllFailed(failures) => {
                f.write_str("all TALA seed attempts failed")?;
                for failure in failures {
                    write!(f, "\nseed {}: ", failure.seed)?;
                    match &failure.error {
                        AttemptError::Failed(error) => write!(f, "{error}")?,
                        AttemptError::Panicked => {
                            f.write_str("TALA seed layout failed due to an internal invariant")?;
                        }
                    }
                }
                Ok(())
            }
        }
    }
}

impl<E: fmt::Debug + fmt::Display> std::error::Error for SelectionError<E> {}

struct Selection<T, E> {
    best: Option<Selected<T>>,
    errors: Vec<Option<SeedFailure<E>>>,
}

impl<T, E> Selection<T, E> {
    fn new(attempts: usize) -> Self {
        Self {
            best: None,
            errors: (0..attempts).map(|_| None).collect(),
        }
    }

    fn consider(&mut self, index: usize, seed: i64, attempt: Result<Scored<T>, AttemptError<E>>) {
        match attempt {
            Err(error) => self.errors[index] = Some(SeedFailure { seed, error }),
            Ok(result)
                if self.best.as_ref().is_none_or(|best| {
                    let order = result.score.compare(best.result.score);
                    order.is_lt() || order.is_eq() && index > best.index
                }) =>
            {
                self.best = Some(Selected {
                    index,
                    seed,
                    result,
                });
            }
            Ok(_) => {}
        }
    }

    fn finish(self, cancelled: bool) -> Result<Selected<T>, SelectionError<E>> {
        if cancelled {
            return Err(SelectionError::Cancelled);
        }
        self.best
            .ok_or_else(|| SelectionError::AllFailed(self.errors.into_iter().flatten().collect()))
    }
}

#[cfg(test)]
mod tests;
