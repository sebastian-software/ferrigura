//! `Array.prototype.sort(comparefn)` as V8 runs it (`src/builtins/array-sort.tq`): TimSort, a port of CPython's
//! `listsort`, with the same comparisons in the same order.
//!
//! The router sorts with comparators that are not consistent orders. For example `|a - b| > 1` decides on the
//! difference and otherwise the index decides, so two elements can compare as equal to a third and unequal to each
//! other. The outcome of a sort then depends on which comparisons it makes, and a different stable sort can give
//! another order. This sort makes the comparisons that V8 makes. A comparator result of NaN counts as zero, as V8
//! converts it.

use std::marker::PhantomData;

/// `kMinGallopWins`: the number of wins in a row that starts galloping, and the initial `minGallop`.
const MIN_GALLOP_WINS: usize = 7;

/// A run on the stack of pending runs: its first index and its length.
#[derive(Clone, Copy)]
struct Run {
    base: usize,
    length: usize,
}

/// How a merge ends: the rest of one run is copied in place, or the rest of the other run is copied to the end.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Exit {
    Succeed,
    CopyRemaining,
}

/// The sort state of one call (`SortState`): the comparator, the pending runs and `minGallop`.
struct Sorter<T, F> {
    compare: F,
    runs: Vec<Run>,
    min_gallop: usize,
    marker: PhantomData<T>,
}

/// Sorts `items` in place as `Array.prototype.sort` does with the comparator `compare`. The comparator returns
/// the number that the JavaScript comparator returns: a negative result puts its first argument first.
pub(crate) fn sort_by_js<T: Copy>(items: &mut [T], compare: impl FnMut(T, T) -> f64) {
    if items.len() < 2 {
        return;
    }
    let mut sorter = Sorter {
        compare,
        runs: Vec::new(),
        min_gallop: MIN_GALLOP_WINS,
        marker: PhantomData,
    };
    sorter.sort(items);
}

/// `ComputeMinRunLength`: the length of the shortest run that the sort builds, for `n` elements.
fn compute_min_run_length(mut n: usize) -> usize {
    let mut r = 0;
    while n >= 64 {
        r |= n & 1;
        n >>= 1;
    }
    n + r
}

impl<T: Copy, F: FnMut(T, T) -> f64> Sorter<T, F> {
    /// The comparator's result as V8 reads it: NaN is zero.
    fn order(&mut self, a: T, b: T) -> f64 {
        let value = (self.compare)(a, b);
        if value.is_nan() { 0.0 } else { value }
    }

    /// `ArrayTimSortImpl`: the runs of the array, each made by `count_and_make_run` and extended by the binary
    /// insertion sort to the minimum length, merged as they come.
    fn sort(&mut self, work: &mut [T]) {
        let mut remaining = work.len();
        let mut low = 0;
        let min_run = compute_min_run_length(remaining);
        while remaining != 0 {
            let mut current = self.count_and_make_run(work, low, low + remaining);
            if current < min_run {
                let forced = min_run.min(remaining);
                self.binary_insertion_sort(work, low, low + current, low + forced);
                current = forced;
            }
            self.runs.push(Run {
                base: low,
                length: current,
            });
            self.merge_collapse(work);
            low += current;
            remaining -= current;
        }
        self.merge_force_collapse(work);
    }

    /// `CountAndMakeRun`: the length of the run that starts at `low_arg`. A strictly descending run is reversed.
    fn count_and_make_run(&mut self, work: &mut [T], low_arg: usize, high: usize) -> usize {
        let low = low_arg + 1;
        if low == high {
            return 1;
        }
        let mut run_length = 2;
        let element_low = work[low];
        let element_low_pre = work[low - 1];
        let mut order = self.order(element_low, element_low_pre);
        let is_descending = order < 0.0;
        let mut previous = element_low;
        for &current in &work[low + 1..high] {
            order = self.order(current, previous);
            if is_descending {
                if order >= 0.0 {
                    break;
                }
            } else if order < 0.0 {
                break;
            }
            previous = current;
            run_length += 1;
        }
        if is_descending {
            work[low_arg..low_arg + run_length].reverse();
        }
        run_length
    }

    /// `BinaryInsertionSort`: inserts the elements from `start_arg` to `high` into the sorted run from `low`.
    fn binary_insertion_sort(&mut self, work: &mut [T], low: usize, start_arg: usize, high: usize) {
        let mut start = if low == start_arg {
            start_arg + 1
        } else {
            start_arg
        };
        while start < high {
            let mut left = low;
            let mut right = start;
            let pivot = work[start];
            while left < right {
                let mid = left + ((right - left) >> 1);
                let mid_value = work[mid];
                let order = self.order(pivot, mid_value);
                if order < 0.0 {
                    right = mid;
                } else {
                    left = mid + 1;
                }
            }
            let mut p = start;
            while p > left {
                work[p] = work[p - 1];
                p -= 1;
            }
            work[left] = pivot;
            start += 1;
        }
    }

    /// `RunInvariantEstablished`: the run `n` on the stack is longer than the sum of the two runs above it.
    fn run_invariant_established(&self, n: usize) -> bool {
        if n < 2 {
            return true;
        }
        self.runs[n - 2].length > self.runs[n - 1].length + self.runs[n].length
    }

    /// `MergeCollapse`: merges the pending runs until the stack keeps its invariant.
    fn merge_collapse(&mut self, work: &mut [T]) {
        while self.runs.len() > 1 {
            let mut n = self.runs.len() - 2;
            if !self.run_invariant_established(n + 1) || !self.run_invariant_established(n) {
                if self.runs[n - 1].length < self.runs[n + 1].length {
                    n -= 1;
                }
                self.merge_at(work, n);
            } else if self.runs[n].length <= self.runs[n + 1].length {
                self.merge_at(work, n);
            } else {
                break;
            }
        }
    }

    /// `MergeForceCollapse`: merges all the pending runs into one.
    fn merge_force_collapse(&mut self, work: &mut [T]) {
        while self.runs.len() > 1 {
            let mut n = self.runs.len() - 2;
            if n > 0 && self.runs[n - 1].length < self.runs[n + 1].length {
                n -= 1;
            }
            self.merge_at(work, n);
        }
    }

    /// `MergeAt`: merges the runs `i` and `i + 1` of the stack, after the parts that are already in place.
    fn merge_at(&mut self, work: &mut [T], i: usize) {
        let stack_size = self.runs.len();
        let Run {
            base: mut base_a,
            length: mut length_a,
        } = self.runs[i];
        let Run {
            base: base_b,
            length: mut length_b,
        } = self.runs[i + 1];
        self.runs[i].length = length_a + length_b;
        if i + 3 == stack_size {
            self.runs[i + 1] = self.runs[i + 2];
        }
        self.runs.pop();

        let key_right = work[base_b];
        let k = self.gallop_right(work, key_right, base_a, length_a, 0);
        base_a += k;
        length_a -= k;
        if length_a == 0 {
            return;
        }
        let key_left = work[base_a + length_a - 1];
        length_b = self.gallop_left(work, key_left, base_b, length_b, length_b - 1);
        if length_b == 0 {
            return;
        }
        if length_a <= length_b {
            self.merge_low(work, base_a, length_a, base_b, length_b);
        } else {
            self.merge_high(work, base_a, length_a, base_b, length_b);
        }
    }

    /// `GallopLeft`: the number of elements of the sorted run at `base` that come before `key`, found by galloping
    /// from `hint`. Equal elements stay before the key.
    fn gallop_left(
        &mut self,
        array: &[T],
        key: T,
        base: usize,
        length: usize,
        hint: usize,
    ) -> usize {
        let (base, length, hint) = (base as isize, length as isize, hint as isize);
        let mut last_ofs: isize = 0;
        let mut offset: isize = 1;
        let hint_element = array[(base + hint) as usize];
        let mut order = self.order(hint_element, key);
        if order < 0.0 {
            // a[hint] < key: gallop right, until a[hint + lastOfs] < key <= a[hint + offset].
            let max_ofs = length - hint;
            while offset < max_ofs {
                let element = array[(base + hint + offset) as usize];
                order = self.order(element, key);
                if order >= 0.0 {
                    break;
                }
                last_ofs = offset;
                offset = (offset << 1) + 1;
            }
            if offset > max_ofs {
                offset = max_ofs;
            }
            last_ofs += hint;
            offset += hint;
        } else {
            // key <= a[hint]: gallop left, until a[hint - offset] < key <= a[hint - lastOfs].
            let max_ofs = hint + 1;
            while offset < max_ofs {
                let element = array[(base + hint - offset) as usize];
                order = self.order(element, key);
                if order < 0.0 {
                    break;
                }
                last_ofs = offset;
                offset = (offset << 1) + 1;
            }
            if offset > max_ofs {
                offset = max_ofs;
            }
            let tmp = last_ofs;
            last_ofs = hint - offset;
            offset = hint - tmp;
        }
        // a[lastOfs] < key <= a[offset]: a binary search.
        last_ofs += 1;
        while last_ofs < offset {
            let m = last_ofs + ((offset - last_ofs) >> 1);
            let element = array[(base + m) as usize];
            order = self.order(element, key);
            if order < 0.0 {
                last_ofs = m + 1;
            } else {
                offset = m;
            }
        }
        offset as usize
    }

    /// `GallopRight`: the number of elements of the sorted run at `base` that come up to and including `key`,
    /// found by galloping from `hint`. Equal elements stay before the key's place.
    fn gallop_right(
        &mut self,
        array: &[T],
        key: T,
        base: usize,
        length: usize,
        hint: usize,
    ) -> usize {
        let (base, length, hint) = (base as isize, length as isize, hint as isize);
        let mut last_ofs: isize = 0;
        let mut offset: isize = 1;
        let hint_element = array[(base + hint) as usize];
        let mut order = self.order(key, hint_element);
        if order < 0.0 {
            // key < a[hint]: gallop left, until a[hint - offset] <= key < a[hint - lastOfs].
            let max_ofs = hint + 1;
            while offset < max_ofs {
                let element = array[(base + hint - offset) as usize];
                order = self.order(key, element);
                if order >= 0.0 {
                    break;
                }
                last_ofs = offset;
                offset = (offset << 1) + 1;
            }
            if offset > max_ofs {
                offset = max_ofs;
            }
            let tmp = last_ofs;
            last_ofs = hint - offset;
            offset = hint - tmp;
        } else {
            // a[hint] <= key: gallop right, until a[hint + lastOfs] <= key < a[hint + offset].
            let max_ofs = length - hint;
            while offset < max_ofs {
                let element = array[(base + hint + offset) as usize];
                order = self.order(key, element);
                if order < 0.0 {
                    break;
                }
                last_ofs = offset;
                offset = (offset << 1) + 1;
            }
            if offset > max_ofs {
                offset = max_ofs;
            }
            last_ofs += hint;
            offset += hint;
        }
        // a[lastOfs] <= key < a[offset]: a binary search.
        last_ofs += 1;
        while last_ofs < offset {
            let m = last_ofs + ((offset - last_ofs) >> 1);
            let element = array[(base + m) as usize];
            order = self.order(key, element);
            if order < 0.0 {
                offset = m;
            } else {
                last_ofs = m + 1;
            }
        }
        offset as usize
    }

    /// `MergeLow`: merges the runs when the first is the shorter. The first run is copied to a temporary array,
    /// and the merge goes forward from its start.
    fn merge_low(
        &mut self,
        work: &mut [T],
        base_a: usize,
        length_a: usize,
        base_b: usize,
        length_b: usize,
    ) {
        let mut length_a = length_a;
        let mut length_b = length_b;
        let temp: Vec<T> = work[base_a..base_a + length_a].to_vec();
        let mut dest = base_a;
        let mut cursor_temp = 0;
        let mut cursor_b = base_b;
        work[dest] = work[cursor_b];
        dest += 1;
        cursor_b += 1;

        let exit = 'merge: {
            length_b -= 1;
            if length_b == 0 {
                break 'merge Exit::Succeed;
            }
            if length_a == 1 {
                break 'merge Exit::CopyRemaining;
            }
            let mut min_gallop = self.min_gallop;
            loop {
                let mut wins_a = 0;
                let mut wins_b = 0;
                // The straightforward merge, until one run appears to win consistently.
                loop {
                    let order = self.order(work[cursor_b], temp[cursor_temp]);
                    if order < 0.0 {
                        work[dest] = work[cursor_b];
                        dest += 1;
                        cursor_b += 1;
                        wins_b += 1;
                        length_b -= 1;
                        wins_a = 0;
                        if length_b == 0 {
                            break 'merge Exit::Succeed;
                        }
                        if wins_b >= min_gallop {
                            break;
                        }
                    } else {
                        work[dest] = temp[cursor_temp];
                        dest += 1;
                        cursor_temp += 1;
                        wins_a += 1;
                        length_a -= 1;
                        wins_b = 0;
                        if length_a == 1 {
                            break 'merge Exit::CopyRemaining;
                        }
                        if wins_a >= min_gallop {
                            break;
                        }
                    }
                }
                // Galloping, while one run keeps winning.
                min_gallop += 1;
                let mut first = true;
                while wins_a >= MIN_GALLOP_WINS || wins_b >= MIN_GALLOP_WINS || first {
                    first = false;
                    min_gallop = if min_gallop > 1 { min_gallop - 1 } else { 1 };
                    self.min_gallop = min_gallop;
                    let key = work[cursor_b];
                    wins_a = self.gallop_right(&temp, key, cursor_temp, length_a, 0);
                    if wins_a > 0 {
                        work[dest..dest + wins_a]
                            .copy_from_slice(&temp[cursor_temp..cursor_temp + wins_a]);
                        dest += wins_a;
                        cursor_temp += wins_a;
                        length_a -= wins_a;
                        if length_a == 1 {
                            break 'merge Exit::CopyRemaining;
                        }
                        if length_a == 0 {
                            break 'merge Exit::Succeed;
                        }
                    }
                    work[dest] = work[cursor_b];
                    dest += 1;
                    cursor_b += 1;
                    length_b -= 1;
                    if length_b == 0 {
                        break 'merge Exit::Succeed;
                    }
                    let key = temp[cursor_temp];
                    wins_b = self.gallop_left(work, key, cursor_b, length_b, 0);
                    if wins_b > 0 {
                        work.copy_within(cursor_b..cursor_b + wins_b, dest);
                        dest += wins_b;
                        cursor_b += wins_b;
                        length_b -= wins_b;
                        if length_b == 0 {
                            break 'merge Exit::Succeed;
                        }
                    }
                    work[dest] = temp[cursor_temp];
                    dest += 1;
                    cursor_temp += 1;
                    length_a -= 1;
                    if length_a == 1 {
                        break 'merge Exit::CopyRemaining;
                    }
                }
                // Penalise the leaving of the galloping mode.
                min_gallop += 1;
                self.min_gallop = min_gallop;
            }
        };

        match exit {
            Exit::Succeed => {
                if length_a > 0 {
                    work[dest..dest + length_a]
                        .copy_from_slice(&temp[cursor_temp..cursor_temp + length_a]);
                }
            }
            Exit::CopyRemaining => {
                // The last element of run A belongs at the end of the merge.
                work.copy_within(cursor_b..cursor_b + length_b, dest);
                work[dest + length_b] = temp[cursor_temp];
            }
        }
    }

    /// `MergeHigh`: merges the runs when the second is the shorter. The second run is copied to a temporary array,
    /// and the merge goes backward from its end.
    fn merge_high(
        &mut self,
        work: &mut [T],
        base_a: usize,
        length_a: usize,
        base_b: usize,
        length_b: usize,
    ) {
        let mut length_a = length_a;
        let mut length_b = length_b;
        let temp: Vec<T> = work[base_b..base_b + length_b].to_vec();
        let mut dest = (base_b + length_b - 1) as isize;
        let mut cursor_temp = (length_b - 1) as isize;
        let mut cursor_a = (base_a + length_a - 1) as isize;
        work[dest as usize] = work[cursor_a as usize];
        dest -= 1;
        cursor_a -= 1;

        let exit = 'merge: {
            length_a -= 1;
            if length_a == 0 {
                break 'merge Exit::Succeed;
            }
            if length_b == 1 {
                break 'merge Exit::CopyRemaining;
            }
            let mut min_gallop = self.min_gallop;
            loop {
                let mut wins_a = 0;
                let mut wins_b = 0;
                loop {
                    let order = self.order(temp[cursor_temp as usize], work[cursor_a as usize]);
                    if order < 0.0 {
                        work[dest as usize] = work[cursor_a as usize];
                        dest -= 1;
                        cursor_a -= 1;
                        wins_a += 1;
                        length_a -= 1;
                        wins_b = 0;
                        if length_a == 0 {
                            break 'merge Exit::Succeed;
                        }
                        if wins_a >= min_gallop {
                            break;
                        }
                    } else {
                        work[dest as usize] = temp[cursor_temp as usize];
                        dest -= 1;
                        cursor_temp -= 1;
                        wins_b += 1;
                        length_b -= 1;
                        wins_a = 0;
                        if length_b == 1 {
                            break 'merge Exit::CopyRemaining;
                        }
                        if wins_b >= min_gallop {
                            break;
                        }
                    }
                }
                min_gallop += 1;
                let mut first = true;
                while wins_a >= MIN_GALLOP_WINS || wins_b >= MIN_GALLOP_WINS || first {
                    first = false;
                    min_gallop = if min_gallop > 1 { min_gallop - 1 } else { 1 };
                    self.min_gallop = min_gallop;
                    let key = temp[cursor_temp as usize];
                    let k = self.gallop_right(work, key, base_a, length_a, length_a - 1);
                    wins_a = length_a - k;
                    if wins_a > 0 {
                        dest -= wins_a as isize;
                        cursor_a -= wins_a as isize;
                        let from = (cursor_a + 1) as usize;
                        work.copy_within(from..from + wins_a, (dest + 1) as usize);
                        length_a -= wins_a;
                        if length_a == 0 {
                            break 'merge Exit::Succeed;
                        }
                    }
                    work[dest as usize] = temp[cursor_temp as usize];
                    dest -= 1;
                    cursor_temp -= 1;
                    length_b -= 1;
                    if length_b == 1 {
                        break 'merge Exit::CopyRemaining;
                    }
                    let key = work[cursor_a as usize];
                    let k = self.gallop_left(&temp, key, 0, length_b, length_b - 1);
                    wins_b = length_b - k;
                    if wins_b > 0 {
                        dest -= wins_b as isize;
                        cursor_temp -= wins_b as isize;
                        let from = (cursor_temp + 1) as usize;
                        work[(dest + 1) as usize..(dest + 1) as usize + wins_b]
                            .copy_from_slice(&temp[from..from + wins_b]);
                        length_b -= wins_b;
                        if length_b == 1 {
                            break 'merge Exit::CopyRemaining;
                        }
                        if length_b == 0 {
                            break 'merge Exit::Succeed;
                        }
                    }
                    work[dest as usize] = work[cursor_a as usize];
                    dest -= 1;
                    cursor_a -= 1;
                    length_a -= 1;
                    if length_a == 0 {
                        break 'merge Exit::Succeed;
                    }
                }
                min_gallop += 1;
                self.min_gallop = min_gallop;
            }
        };

        match exit {
            Exit::Succeed => {
                if length_b > 0 {
                    let start = (dest - (length_b as isize - 1)) as usize;
                    work[start..start + length_b].copy_from_slice(&temp[..length_b]);
                }
            }
            Exit::CopyRemaining => {
                // The first element of run B belongs at the start of the merge; the rest of run A moves up.
                dest -= length_a as isize;
                cursor_a -= length_a as isize;
                let from = (cursor_a + 1) as usize;
                work.copy_within(from..from + length_a, (dest + 1) as usize);
                work[dest as usize] = temp[cursor_temp as usize];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_consistent_order_is_the_stable_sort() {
        // Pairs of (key, index): the key orders them, and equal keys keep the index order.
        let mut items: Vec<(u32, usize)> = (0..500)
            .map(|index| (((index * 7919) % 97) as u32, index))
            .collect();
        let mut expected = items.clone();
        expected.sort_by_key(|&(key, _)| key);
        sort_by_js(&mut items, |a, b| f64::from(a.0) - f64::from(b.0));
        assert_eq!(items, expected);
    }

    #[test]
    fn a_comparator_that_is_nan_is_equal() {
        let mut items = vec![3.0, 1.0, 2.0];
        sort_by_js(&mut items, |a, b| (a - b) * f64::NAN);
        assert_eq!(items, vec![3.0, 1.0, 2.0]);
    }

    #[test]
    fn short_runs_and_descending_runs_are_sorted() {
        let mut items = vec![5, 4, 3, 2, 1];
        sort_by_js(&mut items, |a, b| f64::from(a - b));
        assert_eq!(items, vec![1, 2, 3, 4, 5]);
        let mut items = vec![2, 1];
        sort_by_js(&mut items, |a, b| f64::from(a - b));
        assert_eq!(items, vec![1, 2]);
    }
    /// The indices of `keys`, sorted with the comparator `|a - b| > 1 ? a - b : index order`, which is not a total
    /// order. Each expected order below is V8's, from node 24, for the same comparator.
    fn sorted_by_threshold(keys: &[f64]) -> Vec<usize> {
        let mut items: Vec<(f64, usize)> = keys.iter().enumerate().map(|(i, &k)| (k, i)).collect();
        sort_by_js(&mut items, |a, b| {
            if (a.0 - b.0).abs() > 1.0 {
                a.0 - b.0
            } else {
                a.1 as f64 - b.1 as f64
            }
        });
        items.iter().map(|item| item.1).collect()
    }

    #[test]
    fn a_threshold_comparator_sorts_as_v8_sorts_it() {
        let small = [3.5, 0.0, 0.5, 4.25, 2.0, 2.2, 6.0, 1.0, 1.5, 5.5, 0.25, 3.0];
        assert_eq!(
            sorted_by_threshold(&small),
            vec![1, 2, 10, 4, 7, 5, 8, 0, 11, 3, 6, 9]
        );
        // Seventy keys, which is more than a binary insertion sort takes: the merges run.
        let large: Vec<f64> = (0..70)
            .map(|i| ((i * 37) % 23) as f64 / 4.0 + (i % 5) as f64 * 0.7)
            .collect();
        assert_eq!(
            sorted_by_threshold(&large),
            vec![
                0, 5, 10, 15, 20, 25, 30, 46, 35, 51, 2, 7, 23, 61, 38, 40, 56, 12, 28, 33, 66, 43,
                45, 1, 17, 48, 50, 4, 6, 22, 37, 53, 55, 42, 58, 60, 65, 69, 27, 32, 9, 11, 14, 16,
                47, 63, 21, 68, 26, 3, 19, 8, 24, 13, 29, 31, 18, 34, 36, 52, 57, 67, 39, 41, 62,
                44, 49, 54, 59, 64
            ]
        );
    }
}
