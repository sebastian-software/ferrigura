//! The merge sort behind `Collections.sort` and `Arrays.sort` in GWT.
//!
//! GWT does not use TimSort. It uses the legacy merge sort: insertion sort
//! below seven elements, a shortcut when the two halves are already ordered,
//! and a merge that prefers the left run on ties. Comparator results are
//! observed in the same order, so an inconsistent comparator yields the same
//! output as in elkjs.
//!
//! Source: `mergeSort`, `mergeSort_0`, `merge_1` and `insertionSort` in
//! `lib/elk-worker.js` of elkjs 0.9.3.

use std::cmp::Ordering;

/// Sorts `array` in place, the way `Collections.sort(list, comparator)` does in GWT.
pub fn sort_by<T: Clone, F>(array: &mut [T], mut comp: F)
where
    F: FnMut(&T, &T) -> Ordering,
{
    let high = array.len();
    let mut temp: Vec<T> = array.to_vec();
    merge_sort(&mut temp, array, 0, high, &mut comp);
}

/// `mergeSort_0` with a zero offset: the top-level call always starts at zero.
fn merge_sort<T: Clone, F>(temp: &mut [T], array: &mut [T], low: usize, high: usize, comp: &mut F)
where
    F: FnMut(&T, &T) -> Ordering,
{
    if high - low < 7 {
        insertion_sort(array, low, high, comp);
        return;
    }
    let temp_low = low;
    let temp_high = high;
    let temp_mid = temp_low + ((temp_high - temp_low) >> 1);

    // The halves are sorted into `temp`, so the roles of the two buffers swap.
    merge_sort(array, temp, temp_low, temp_mid, comp);
    merge_sort(array, temp, temp_mid, temp_high, comp);

    if comp(&temp[temp_mid - 1], &temp[temp_mid]) != Ordering::Greater {
        for index in low..high {
            array[index] = temp[temp_low + index - low].clone();
        }
        return;
    }
    merge(temp, temp_low, temp_mid, temp_high, array, low, high, comp);
}

fn insertion_sort<T: Clone, F>(array: &mut [T], low: usize, high: usize, comp: &mut F)
where
    F: FnMut(&T, &T) -> Ordering,
{
    for i in low + 1..high {
        let mut j = i;
        while j > low && comp(&array[j - 1], &array[j]) == Ordering::Greater {
            array.swap(j, j - 1);
            j -= 1;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn merge<T: Clone, F>(
    src: &[T],
    src_low: usize,
    src_mid: usize,
    src_high: usize,
    dest: &mut [T],
    dest_low: usize,
    dest_high: usize,
    comp: &mut F,
) where
    F: FnMut(&T, &T) -> Ordering,
{
    let mut src_low = src_low;
    let mut top_idx = src_mid;
    let mut dest_index = dest_low;
    while dest_index < dest_high {
        let take_low = top_idx >= src_high
            || (src_low < src_mid && comp(&src[src_low], &src[top_idx]) != Ordering::Greater);
        if take_low {
            dest[dest_index] = src[src_low].clone();
            src_low += 1;
        } else {
            dest[dest_index] = src[top_idx].clone();
            top_idx += 1;
        }
        dest_index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_like_a_stable_sort_for_a_consistent_comparator() {
        let mut values: Vec<(i32, usize)> = (0..200).map(|i| ((i * 37 % 11) as i32, i)).collect();
        let mut expected = values.clone();
        expected.sort_by_key(|pair| pair.0);
        sort_by(&mut values, |a, b| a.0.cmp(&b.0));
        assert_eq!(values, expected);
    }

    #[test]
    fn matches_elkjs_for_an_inconsistent_comparator() {
        // Reference output from elkjs 0.9.3's GWT `sort_5` with the same comparator.
        let mut values: Vec<i64> = vec![
            5, 3, 9, 1, 7, 2, 8, 6, 4, 0, 11, 10, 15, 13, 12, 14, 19, 18, 17, 16, 21, 20, 23, 22,
            25, 24, 27, 26, 29, 28,
        ];
        sort_by(&mut values, |a, b| (((a * 31 + b * 17) % 3) - 1).cmp(&0));
        assert_eq!(
            values,
            vec![
                3, 9, 5, 2, 8, 1, 7, 4, 6, 0, 11, 10, 13, 15, 12, 14, 19, 18, 17, 16, 21, 20, 23,
                22, 25, 24, 27, 26, 29, 28
            ]
        );
    }

    #[test]
    fn short_inputs_use_insertion_sort() {
        let mut values = vec![3, 1, 2];
        sort_by(&mut values, |a, b| a.cmp(b));
        assert_eq!(values, vec![1, 2, 3]);
    }
}
