//! `js::js_sort_by` against V8's `Array.prototype.sort` (Node v24.21.0, V8 13.6): the permutation of every case,
//! and the sequence of comparator calls, as a hash for every case and in full for the short ones.
//!
//! The expected values are in `tests/data/js-sort.txt`, written by `tools/js-sort.ts`; its first comments give
//! the format, the comparators and the key sets. The keys of each case are drawn again here from the case's
//! seed, with the same arithmetic as the generator, so the data holds no keys. Each test reports every
//! mismatch, not just the first.
//!
//! The data is node's (V8 13.6). Chromium 151 gives other results for the inconsistent comparators; the
//! generator's header says where they differ.

use std::cell::RefCell;
use std::path::PathBuf;

use ferrigura_swimlane::js::js_sort_by;

/// The keys of the `special` key set, in the order the generator draws them.
const SPECIAL: [f64; 10] = [
    f64::NAN,
    0.0,
    -0.0,
    f64::INFINITY,
    f64::NEG_INFINITY,
    1.0,
    2.0,
    3.0,
    -1.0,
    0.5,
];

/// The number of cases in the data file (4 key sets, 3 comparators, the lengths 0 to 300, 500, 1000, 3000).
const CASES: usize = 3 * 4 * (301 + 3);

/// The number of cases with a full log: the lengths 0 to 10, for each of the 12 combinations.
const LOGS: usize = 3 * 4 * 11;

/// mulberry32, as the generator draws it.
struct Mulberry32(u32);

impl Mulberry32 {
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_add(0x6d2b_79f5);
        let a = self.0;
        let mut t = (a ^ (a >> 15)).wrapping_mul(1 | a);
        t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t)) ^ t;
        t ^ (t >> 14)
    }

    /// The generator's `rand()`: a uniform number in [0, 1).
    fn unit(&mut self) -> f64 {
        f64::from(self.next_u32()) / 4_294_967_296.0
    }
}

/// The keys of a case, as `tools/js-sort.ts` makes them.
fn keys_of(key_set: &str, n: usize, seed: u32) -> Vec<f64> {
    let mut rand = Mulberry32(seed);
    let mut keys = Vec::with_capacity(n);
    if key_set == "runs" {
        let mut value = ((rand.unit() - 0.5) * 2000.0).floor();
        let mut descending = false;
        let mut remaining = 0_usize;
        for _ in 0..n {
            if remaining == 0 {
                remaining = 1 + (rand.unit() * 200.0).floor() as usize;
                descending = rand.unit() < 0.25;
            }
            remaining -= 1;
            if rand.unit() < 1.0 / 16.0 {
                keys.push(f64::NAN);
            } else {
                let step = (rand.unit() * 3.0).floor();
                value = if descending {
                    value - step
                } else {
                    value + step
                };
                keys.push(value);
            }
        }
        return keys;
    }
    for _ in 0..n {
        let key = match key_set {
            "special" => SPECIAL[(rand.unit() * SPECIAL.len() as f64).floor() as usize],
            "doubles" => (rand.unit() - 0.5) * 2000.0,
            "ties" => {
                if rand.unit() < 1.0 / 8.0 {
                    f64::NAN
                } else {
                    (rand.unit() * 9.0).floor() - 4.0
                }
            }
            other => panic!("unknown key set {other:?}"),
        };
        keys.push(key);
    }
    keys
}

/// The hash of the generator's `pairHash`: a mulberry32 step over the seed and the two ids.
fn pair_hash(seed: u32, a: u32, b: u32) -> u32 {
    let mut t = seed ^ a.wrapping_mul(0x9e37_79b1) ^ b.wrapping_mul(0x85eb_ca77);
    t = t.wrapping_add(0x6d2b_79f5);
    t = (t ^ (t >> 15)).wrapping_mul(1 | t);
    t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t)) ^ t;
    t ^ (t >> 14)
}

/// The value the comparator of the case returns for the ids `a` and `b`, as the generator's comparators do.
fn compare_pair(kind: &str, keys: &[f64], seed: u32, a: u32, b: u32) -> f64 {
    let (key_a, key_b) = (keys[a as usize], keys[b as usize]);
    match kind {
        "diff" => key_a - key_b,
        "heuristic" => {
            if key_a.is_nan() || key_b.is_nan() {
                if a < b { -1.0 } else { 1.0 }
            } else {
                key_a - key_b
            }
        }
        "hash" => f64::from(pair_hash(seed, a, b) % 3) - 1.0,
        other => panic!("unknown comparator {other:?}"),
    }
}

/// The hash of a sequence of calls, as `callHash` in the generator: two 32-bit lanes over each call's code.
fn call_hash(calls: &[(u32, u32)]) -> String {
    let mut h1: u32 = 0x811c_9dc5;
    let mut h2: u32 = 0x6a09_e667;
    for &(a, b) in calls {
        let code = (a << 16) | b;
        h1 = (h1 ^ code).wrapping_mul(0x0100_0193);
        h2 = (h2 ^ code).wrapping_mul(0x9e37_79b1);
        h2 ^= h2 >> 16;
    }
    format!("{h1:08x}{h2:08x}")
}

/// Sorts the ids 0..n of a case with its comparator, and returns the permutation and every call, in order.
fn sort_case(kind: &str, key_set: &str, n: usize, seed: u32) -> (Vec<u32>, Vec<(u32, u32)>) {
    let keys = keys_of(key_set, n, seed);
    let calls = RefCell::new(Vec::new());
    let mut ids: Vec<u32> = (0..n as u32).collect();
    js_sort_by(&mut ids, |a, b| {
        calls.borrow_mut().push((a, b));
        compare_pair(kind, &keys, seed, a, b)
    });
    (ids, calls.into_inner())
}

fn data(name: &str) -> String {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "data", name]
        .iter()
        .collect();
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The data lines, without the comment lines.
fn rows(text: &str) -> impl Iterator<Item = &str> {
    text.lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
}

/// The first calls of a sequence, for a message.
fn head(calls: &[(u32, u32)]) -> String {
    calls
        .iter()
        .take(12)
        .map(|(a, b)| format!("{a}:{b}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Reports the failures, if there are any, with a count and the first twenty.
fn report(what: &str, failures: &[String]) {
    assert!(
        failures.is_empty(),
        "{what}: {} mismatches with V8; the first ones:\n{}",
        failures.len(),
        failures
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn js_sort_by_matches_v8_for_every_case() {
    let text = data("js-sort.txt");
    let mut cases = 0;
    let mut logs = 0;
    let mut failures = Vec::new();
    for line in rows(&text) {
        let fields: Vec<&str> = line.split(' ').collect();
        match fields.as_slice() {
            ["case", kind, key_set, n, seed, calls, hash, permutation] => {
                cases += 1;
                let n: usize = n.parse().expect("n");
                let seed: u32 = seed.parse().expect("seed");
                let expected_calls: usize = calls.parse().expect("calls");
                let expected: Vec<u32> = if *permutation == "-" {
                    Vec::new()
                } else {
                    permutation
                        .split(',')
                        .map(|id| id.parse().expect("id"))
                        .collect()
                };
                let (got, log) = sort_case(kind, key_set, n, seed);
                let got_hash = call_hash(&log);
                if got != expected || log.len() != expected_calls || got_hash != *hash {
                    failures.push(format!(
                        "case {kind} {key_set} n={n} seed={seed}: calls {} (V8 {expected_calls}), \
                         hash {got_hash} (V8 {hash}), permutation {}; the first calls: {}",
                        log.len(),
                        if got == expected { "as V8" } else { "differs" },
                        head(&log),
                    ));
                }
            }
            ["log", kind, key_set, n, seed, pairs @ ..] => {
                logs += 1;
                let n: usize = n.parse().expect("n");
                let seed: u32 = seed.parse().expect("seed");
                let expected: Vec<(u32, u32)> = pairs
                    .iter()
                    .map(|pair| {
                        let (a, b) = pair.split_once(':').expect("a:b");
                        (a.parse().expect("a"), b.parse().expect("b"))
                    })
                    .collect();
                let (_, log) = sort_case(kind, key_set, n, seed);
                if log != expected {
                    failures.push(format!(
                        "log {kind} {key_set} n={n} seed={seed}: calls {}, V8 {}; the first calls: {}, V8: {}",
                        log.len(),
                        expected.len(),
                        head(&log),
                        head(&expected),
                    ));
                }
            }
            _ => panic!("unexpected line {line:?}"),
        }
    }
    assert_eq!(cases, CASES, "the number of cases in the data file");
    assert_eq!(logs, LOGS, "the number of logs in the data file");
    report("js_sort_by", &failures);
}
