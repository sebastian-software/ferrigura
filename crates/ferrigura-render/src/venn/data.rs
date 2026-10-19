//! The subsets the renderer works on: the model's subsets, the synthetic pairs of
//! `ensurePairwiseSubsets` (`vennRenderer.ts` 377-435), and the JavaScript helpers the reference's
//! arithmetic depends on (`Math.min` and `Math.max` with NaN, the default `sort` of strings).

use std::collections::{HashMap, HashSet};

use ferrigura_model::venn::Subset;

/// A subset as the renderer holds it: its sets (sorted), its size, and its label (empty for none,
/// which the reference reads as `undefined`).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Datum {
    pub sets: Vec<String>,
    pub size: f64,
    pub label: String,
}

impl Datum {
    /// The key of the datum's sets for the lookups by sets: the sorted ids joined with `|`
    /// (`stableSetsKey`, `vennRenderer.ts` 243-245).
    pub fn key(&self) -> String {
        stable_key(&self.sets)
    }
}

/// `stableSetsKey(setIds)`: the ids joined with `|`.
pub(super) fn stable_key(sets: &[String]) -> String {
    sets.join("|")
}

/// The datums of a diagram's subsets, in the order the statements gave them.
pub(super) fn datums(subsets: &[Subset]) -> Vec<Datum> {
    subsets
        .iter()
        .map(|subset| Datum {
            sets: subset.sets.clone(),
            size: subset.size,
            label: subset.label.clone(),
        })
        .collect()
}

/// `ensurePairwiseSubsets(subsets)` (`vennRenderer.ts` 393-435): for every union of three or more
/// sets, the pairs of its sets that are not subsets yet, appended in order. A pair's size is a
/// quarter of the smaller of its two sets' sizes, or 2.5 when one of them has no single-set
/// subset. The synthetic pairs carry no label.
pub(super) fn ensure_pairwise_subsets(subsets: &[Datum]) -> Vec<Datum> {
    let mut existing: HashSet<String> = subsets
        .iter()
        .map(|subset| stable_key(&sorted(&subset.sets)))
        .collect();

    // The sizes of the single sets: a later statement for the same set replaces the earlier one.
    let mut sizes: HashMap<&str, f64> = HashMap::new();
    for subset in subsets {
        if subset.sets.len() == 1 {
            sizes.insert(subset.sets[0].as_str(), subset.size);
        }
    }

    let mut synthetic = Vec::new();
    for subset in subsets {
        if subset.sets.len() < 3 {
            continue;
        }
        let members = sorted(&subset.sets);
        for i in 0..members.len() - 1 {
            for j in i + 1..members.len() {
                let pair = vec![members[i].clone(), members[j].clone()];
                let key = stable_key(&pair);
                if existing.contains(&key) {
                    continue;
                }
                existing.insert(key);
                let size = match (sizes.get(pair[0].as_str()), sizes.get(pair[1].as_str())) {
                    (Some(&a), Some(&b)) => js_min(a, b) / 4.0,
                    _ => 2.5,
                };
                synthetic.push(Datum {
                    sets: pair,
                    size,
                    label: String::new(),
                });
            }
        }
    }

    let mut all = subsets.to_vec();
    all.extend(synthetic);
    all
}

/// `[...sets].sort()`: the default sort of JavaScript, by UTF-16 code units.
pub(super) fn sorted(sets: &[String]) -> Vec<String> {
    let mut sorted = sets.to_vec();
    sorted.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    sorted
}

/// `Math.min(a, b)`: NaN if either is NaN, and the negative zero of the two if they are equal.
pub(super) fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == b {
        if a.is_sign_negative() { a } else { b }
    } else if a < b {
        a
    } else {
        b
    }
}

/// `Math.max(a, b)`: NaN if either is NaN, and the positive zero of the two if they are equal.
pub(super) fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == b {
        if a.is_sign_negative() { b } else { a }
    } else if a > b {
        a
    } else {
        b
    }
}

/// `Math.min(...values)` of a list: `Infinity` for an empty list.
pub(super) fn js_min_all(values: impl IntoIterator<Item = f64>) -> f64 {
    values.into_iter().fold(f64::INFINITY, js_min)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(sets: &[&str], size: f64) -> Datum {
        Datum {
            sets: sets.iter().map(|s| (*s).to_owned()).collect(),
            size,
            label: String::new(),
        }
    }

    #[test]
    fn js_min_and_max_propagate_nan_and_order_zeros_as_javascript_does() {
        assert!(js_min(f64::NAN, 1.0).is_nan());
        assert!(js_max(1.0, f64::NAN).is_nan());
        assert!(js_min(0.0, -0.0).is_sign_negative());
        assert!(!js_max(-0.0, 0.0).is_sign_negative());
        assert_eq!(js_min_all([3.0, 1.0, 2.0]), 1.0);
        assert_eq!(js_min_all(std::iter::empty()), f64::INFINITY);
    }

    #[test]
    fn a_three_set_union_adds_its_missing_pairs_in_order() {
        let subsets = vec![
            set(&["A"], 10.0),
            set(&["B"], 8.0),
            set(&["C"], 6.0),
            set(&["A", "B", "C"], 2.0),
        ];
        let all = ensure_pairwise_subsets(&subsets);
        let pairs: Vec<Vec<String>> = all[4..].iter().map(|d| d.sets.clone()).collect();
        assert_eq!(
            pairs,
            vec![
                vec!["A".to_owned(), "B".to_owned()],
                vec!["A".to_owned(), "C".to_owned()],
                vec!["B".to_owned(), "C".to_owned()],
            ]
        );
        assert_eq!(all[4].size, 2.0);
        assert_eq!(all[5].size, 1.5);
        assert_eq!(all[6].size, 1.5);
    }

    #[test]
    fn without_three_set_unions_the_list_is_unchanged() {
        let subsets = vec![set(&["A"], 10.0), set(&["A", "B"], 2.0)];
        assert_eq!(ensure_pairwise_subsets(&subsets), subsets);
        assert!(ensure_pairwise_subsets(&[]).is_empty());
    }

    #[test]
    fn partial_coverage_adds_only_the_missing_pairs() {
        // `vennRenderer.spec.ts`: the pair A|B and B|C exist, so only A|C is added.
        let subsets = vec![
            set(&["A"], 10.0),
            set(&["B"], 10.0),
            set(&["C"], 10.0),
            set(&["A", "B", "C"], 5.0),
            set(&["A", "B"], 2.5),
            set(&["B", "C"], 2.5),
        ];
        let all = ensure_pairwise_subsets(&subsets);
        assert_eq!(all.len(), 7);
        let added = &all[6];
        assert_eq!(added.sets, ["A", "C"]);
        assert_eq!(added.size, 2.5);
    }

    #[test]
    fn sets_out_of_alphabetical_order_give_sorted_pairs() {
        // The union is `B, A, C`: its pairs are added sorted, as A|B, A|C and B|C.
        let subsets = vec![
            set(&["A"], 10.0),
            set(&["B"], 10.0),
            set(&["C"], 10.0),
            set(&["B", "A", "C"], 5.0),
        ];
        let all = ensure_pairwise_subsets(&subsets);
        assert_eq!(all.len(), 7);
        let keys: Vec<String> = all[4..].iter().map(Datum::key).collect();
        assert_eq!(keys, ["A|B", "A|C", "B|C"]);
    }

    #[test]
    fn a_pair_falls_back_to_size_two_and_a_half_without_single_sizes() {
        // With no single-set subset of A, B or C, every pair takes the default of 2.5.
        let subsets = vec![set(&["A", "B", "C"], 5.0)];
        let all = ensure_pairwise_subsets(&subsets);
        assert_eq!(all.len(), 4);
        assert!(all[1..].iter().all(|pair| pair.size == 2.5));
    }
}
