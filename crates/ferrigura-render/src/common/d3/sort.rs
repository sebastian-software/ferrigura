//! `Array.prototype.sort` with a comparator, as V8 runs it: TimSort, after CPython's `listsort`
//! (`third_party/v8/builtins/array-sort.tq`). A comparator result that is NaN counts as `0`
//! (`SortCompare` of ECMA-262). The sankey layout sorts with `a.y0 - b.y0`, which is NaN for a
//! position that is NaN or infinite, so two items can be "equal" to each other and "less" than a
//! third; the order that comes out then depends on the comparisons the algorithm makes, and the port
//! makes them in V8's order and with its arguments.
//!
//! For fewer than 64 items V8 takes the whole array as one run: a run of the first two items' order
//! followed by a binary insertion sort. The merges of longer arrays follow the same steps.

/// `kMinGallopWins` of V8: the number of wins in a row that starts galloping.
const MIN_GALLOP: isize = 7;

/// Sorts `items` in place with `compare`, which returns the number the JavaScript comparator would.
/// The sort is stable for a consistent comparator.
pub fn sort_js<T: Copy>(items: &mut [T], compare: impl FnMut(T, T) -> f64) {
    if items.len() < 2 {
        return;
    }
    let mut sorter = Sorter {
        work: items,
        temp: Vec::new(),
        compare,
        min_gallop: MIN_GALLOP,
        runs: Vec::new(),
    };
    sorter.run();
}

/// The comparator's result as V8 reads it: NaN is `0`.
fn order<T: Copy, F: FnMut(T, T) -> f64>(compare: &mut F, a: T, b: T) -> f64 {
    let value = compare(a, b);
    if value.is_nan() { 0.0 } else { value }
}

/// `ComputeMinRunLength`: the length of the shortest run TimSort makes.
fn min_run_length(length: usize) -> usize {
    let mut n = length;
    let mut r = 0;
    while n >= 64 {
        r |= n & 1;
        n >>= 1;
    }
    n + r
}

struct Sorter<'a, T, F> {
    work: &'a mut [T],
    temp: Vec<T>,
    compare: F,
    min_gallop: isize,
    /// The pending runs as `(base, length)`, the bottom first.
    runs: Vec<(usize, usize)>,
}

enum Exit {
    Succeed,
    Copy,
}

impl<T: Copy, F: FnMut(T, T) -> f64> Sorter<'_, T, F> {
    /// `ArrayTimSortImpl`: the runs of the array, each made to `minrun` by binary insertion, and
    /// merged as the pending runs require.
    fn run(&mut self) {
        let length = self.work.len();
        let min_run = min_run_length(length);
        let mut remaining = length;
        let mut low = 0;
        while remaining != 0 {
            let mut current = self.count_and_make_run(low, low + remaining);
            if current < min_run {
                let forced = min_run.min(remaining);
                self.binary_insertion_sort(low, low + current, low + forced);
                current = forced;
            }
            self.runs.push((low, current));
            self.merge_collapse();
            low += current;
            remaining -= current;
        }
        self.merge_force_collapse();
    }

    /// `CountAndMakeRun`: the length of the run that starts at `low_arg`, reversed when it is
    /// strictly descending.
    fn count_and_make_run(&mut self, low_arg: usize, high: usize) -> usize {
        let low = low_arg + 1;
        if low == high {
            return 1;
        }
        let mut run_length = 2;
        let element_low = self.work[low];
        let element_low_pre = self.work[low - 1];
        let is_descending = order(&mut self.compare, element_low, element_low_pre) < 0.0;
        let mut previous = element_low;
        for index in (low + 1)..high {
            let current = self.work[index];
            let value = order(&mut self.compare, current, previous);
            let stop = if is_descending {
                value >= 0.0
            } else {
                value < 0.0
            };
            if stop {
                break;
            }
            previous = current;
            run_length += 1;
        }
        if is_descending {
            self.work[low_arg..low_arg + run_length].reverse();
        }
        run_length
    }

    /// `BinaryInsertionSort(low, start, high)`: the items from `start` on are inserted in order,
    /// each at the place a binary search finds after the sorted part that starts at `low`.
    fn binary_insertion_sort(&mut self, low: usize, start_arg: usize, high: usize) {
        let mut start = if low == start_arg {
            start_arg + 1
        } else {
            start_arg
        };
        while start < high {
            let pivot = self.work[start];
            let mut left = low;
            let mut right = start;
            while left < right {
                let mid = left + ((right - left) >> 1);
                let mid_value = self.work[mid];
                if order(&mut self.compare, pivot, mid_value) < 0.0 {
                    right = mid;
                } else {
                    left = mid + 1;
                }
            }
            let mut position = start;
            while position > left {
                self.work[position] = self.work[position - 1];
                position -= 1;
            }
            self.work[left] = pivot;
            start += 1;
        }
    }

    /// `RunInvariantEstablished`: the pending run `n` is longer than the run above it and the one
    /// above that together.
    fn run_invariant(&self, n: usize) -> bool {
        if n < 2 {
            return true;
        }
        let a = self.runs[n - 2].1;
        let b = self.runs[n - 1].1;
        let c = self.runs[n].1;
        a > b + c
    }

    /// `MergeCollapse`: merges the top pending runs while the run invariant fails.
    fn merge_collapse(&mut self) {
        while self.runs.len() > 1 {
            let mut n = self.runs.len() - 2;
            if !self.run_invariant(n + 1) || !self.run_invariant(n) {
                if self.runs[n - 1].1 < self.runs[n + 1].1 {
                    n -= 1;
                }
                self.merge_at(n);
            } else if self.runs[n].1 <= self.runs[n + 1].1 {
                self.merge_at(n);
            } else {
                break;
            }
        }
    }

    /// `MergeForceCollapse`: merges every pending run.
    fn merge_force_collapse(&mut self) {
        while self.runs.len() > 1 {
            let mut n = self.runs.len() - 2;
            if n > 0 && self.runs[n - 1].1 < self.runs[n + 1].1 {
                n -= 1;
            }
            self.merge_at(n);
        }
    }

    /// `MergeAt(i)`: merges the pending runs `i` and `i + 1`, after the items of the first run that
    /// are already in place and the items of the second run that are already in place are left out.
    fn merge_at(&mut self, i: usize) {
        let (mut base_a, mut length_a) = self.runs[i];
        let (base_b, mut length_b) = self.runs[i + 1];
        self.runs[i].1 = length_a + length_b;
        if i + 3 == self.runs.len() {
            self.runs[i + 1] = self.runs[i + 2];
        }
        self.runs.pop();

        let key_right = self.work[base_b];
        let skip = gallop_right(
            &mut self.compare,
            &*self.work,
            key_right,
            base_a as isize,
            length_a as isize,
            0,
        );
        base_a += skip as usize;
        length_a -= skip as usize;
        if length_a == 0 {
            return;
        }

        let key_left = self.work[base_a + length_a - 1];
        length_b = gallop_left(
            &mut self.compare,
            &*self.work,
            key_left,
            base_b as isize,
            length_b as isize,
            length_b as isize - 1,
        ) as usize;
        if length_b == 0 {
            return;
        }

        if length_a <= length_b {
            self.merge_low(base_a, length_a, base_b, length_b);
        } else {
            self.merge_high(base_a, length_a, base_b, length_b);
        }
    }

    /// `MergeLow`: the first run is the shorter one. It is copied to the temporary array and merged
    /// from the left.
    fn merge_low(&mut self, base_a: usize, length_a: usize, base_b: usize, length_b: usize) {
        let Self {
            work,
            temp,
            compare,
            min_gallop,
            ..
        } = self;
        temp.clear();
        temp.extend_from_slice(&work[base_a..base_a + length_a]);
        let mut length_a = length_a as isize;
        let mut length_b = length_b as isize;
        let mut dest = base_a as isize;
        let mut cursor_temp: isize = 0;
        let mut cursor_b = base_b as isize;

        work[dest as usize] = work[cursor_b as usize];
        dest += 1;
        cursor_b += 1;

        let exit = 'merge: {
            length_b -= 1;
            if length_b == 0 {
                break 'merge Exit::Succeed;
            }
            if length_a == 1 {
                break 'merge Exit::Copy;
            }
            let mut gallop = *min_gallop;
            loop {
                let mut wins_a: isize = 0;
                let mut wins_b: isize = 0;
                // The straightforward merge, until one run wins often enough in a row.
                loop {
                    let b_value = work[cursor_b as usize];
                    let a_value = temp[cursor_temp as usize];
                    if order(compare, b_value, a_value) < 0.0 {
                        work[dest as usize] = b_value;
                        dest += 1;
                        cursor_b += 1;
                        wins_b += 1;
                        length_b -= 1;
                        wins_a = 0;
                        if length_b == 0 {
                            break 'merge Exit::Succeed;
                        }
                        if wins_b >= gallop {
                            break;
                        }
                    } else {
                        work[dest as usize] = a_value;
                        dest += 1;
                        cursor_temp += 1;
                        wins_a += 1;
                        length_a -= 1;
                        wins_b = 0;
                        if length_a == 1 {
                            break 'merge Exit::Copy;
                        }
                        if wins_a >= gallop {
                            break;
                        }
                    }
                }
                // Galloping: each run takes the items it wins by a search.
                gallop += 1;
                let mut first = true;
                while wins_a >= MIN_GALLOP || wins_b >= MIN_GALLOP || first {
                    first = false;
                    gallop = if gallop > 1 { gallop - 1 } else { 1 };
                    *min_gallop = gallop;

                    let key = work[cursor_b as usize];
                    wins_a = gallop_right(compare, &temp[..], key, cursor_temp, length_a, 0);
                    if wins_a > 0 {
                        let count = wins_a as usize;
                        let from = cursor_temp as usize;
                        work[dest as usize..dest as usize + count]
                            .copy_from_slice(&temp[from..from + count]);
                        dest += wins_a;
                        cursor_temp += wins_a;
                        length_a -= wins_a;
                        if length_a == 1 {
                            break 'merge Exit::Copy;
                        }
                        if length_a == 0 {
                            break 'merge Exit::Succeed;
                        }
                    }
                    work[dest as usize] = work[cursor_b as usize];
                    dest += 1;
                    cursor_b += 1;
                    length_b -= 1;
                    if length_b == 0 {
                        break 'merge Exit::Succeed;
                    }

                    let key = temp[cursor_temp as usize];
                    wins_b = gallop_left(compare, &work[..], key, cursor_b, length_b, 0);
                    if wins_b > 0 {
                        let count = wins_b as usize;
                        work.copy_within(
                            cursor_b as usize..cursor_b as usize + count,
                            dest as usize,
                        );
                        dest += wins_b;
                        cursor_b += wins_b;
                        length_b -= wins_b;
                        if length_b == 0 {
                            break 'merge Exit::Succeed;
                        }
                    }
                    work[dest as usize] = temp[cursor_temp as usize];
                    dest += 1;
                    cursor_temp += 1;
                    length_a -= 1;
                    if length_a == 1 {
                        break 'merge Exit::Copy;
                    }
                }
                // Leaving the galloping mode is penalised.
                gallop += 1;
                *min_gallop = gallop;
            }
        };

        match exit {
            Exit::Succeed => {
                if length_a > 0 {
                    let count = length_a as usize;
                    let from = cursor_temp as usize;
                    work[dest as usize..dest as usize + count]
                        .copy_from_slice(&temp[from..from + count]);
                }
            }
            Exit::Copy => {
                let count = length_b as usize;
                work.copy_within(cursor_b as usize..cursor_b as usize + count, dest as usize);
                work[(dest + length_b) as usize] = temp[cursor_temp as usize];
            }
        }
    }

    /// `MergeHigh`: the second run is the shorter one. It is copied to the temporary array and
    /// merged from the right.
    fn merge_high(&mut self, base_a: usize, length_a: usize, base_b: usize, length_b: usize) {
        let Self {
            work,
            temp,
            compare,
            min_gallop,
            ..
        } = self;
        temp.clear();
        temp.extend_from_slice(&work[base_b..base_b + length_b]);
        let mut length_a = length_a as isize;
        let mut length_b = length_b as isize;
        let mut dest = (base_b + length_b as usize) as isize - 1;
        let mut cursor_temp = length_b - 1;
        let mut cursor_a = (base_a + length_a as usize) as isize - 1;

        work[dest as usize] = work[cursor_a as usize];
        dest -= 1;
        cursor_a -= 1;

        let exit = 'merge: {
            length_a -= 1;
            if length_a == 0 {
                break 'merge Exit::Succeed;
            }
            if length_b == 1 {
                break 'merge Exit::Copy;
            }
            let mut gallop = *min_gallop;
            loop {
                let mut wins_a: isize = 0;
                let mut wins_b: isize = 0;
                loop {
                    let b_value = temp[cursor_temp as usize];
                    let a_value = work[cursor_a as usize];
                    if order(compare, b_value, a_value) < 0.0 {
                        work[dest as usize] = a_value;
                        dest -= 1;
                        cursor_a -= 1;
                        wins_a += 1;
                        length_a -= 1;
                        wins_b = 0;
                        if length_a == 0 {
                            break 'merge Exit::Succeed;
                        }
                        if wins_a >= gallop {
                            break;
                        }
                    } else {
                        work[dest as usize] = b_value;
                        dest -= 1;
                        cursor_temp -= 1;
                        wins_b += 1;
                        length_b -= 1;
                        wins_a = 0;
                        if length_b == 1 {
                            break 'merge Exit::Copy;
                        }
                        if wins_b >= gallop {
                            break;
                        }
                    }
                }
                gallop += 1;
                let mut first = true;
                while wins_a >= MIN_GALLOP || wins_b >= MIN_GALLOP || first {
                    first = false;
                    gallop = if gallop > 1 { gallop - 1 } else { 1 };
                    *min_gallop = gallop;

                    let key = temp[cursor_temp as usize];
                    let k = gallop_right(
                        compare,
                        &work[..],
                        key,
                        base_a as isize,
                        length_a,
                        length_a - 1,
                    );
                    wins_a = length_a - k;
                    if wins_a > 0 {
                        dest -= wins_a;
                        cursor_a -= wins_a;
                        let count = wins_a as usize;
                        work.copy_within(
                            (cursor_a + 1) as usize..(cursor_a + 1) as usize + count,
                            (dest + 1) as usize,
                        );
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
                        break 'merge Exit::Copy;
                    }

                    let key = work[cursor_a as usize];
                    let k = gallop_left(compare, &temp[..], key, 0, length_b, length_b - 1);
                    wins_b = length_b - k;
                    if wins_b > 0 {
                        dest -= wins_b;
                        cursor_temp -= wins_b;
                        let count = wins_b as usize;
                        let from = (cursor_temp + 1) as usize;
                        work[(dest + 1) as usize..(dest + 1) as usize + count]
                            .copy_from_slice(&temp[from..from + count]);
                        length_b -= wins_b;
                        if length_b == 1 {
                            break 'merge Exit::Copy;
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
                gallop += 1;
                *min_gallop = gallop;
            }
        };

        match exit {
            Exit::Succeed => {
                if length_b > 0 {
                    let count = length_b as usize;
                    let start = (dest - (length_b - 1)) as usize;
                    work[start..start + count].copy_from_slice(&temp[..count]);
                }
            }
            Exit::Copy => {
                dest -= length_a;
                cursor_a -= length_a;
                let count = length_a as usize;
                work.copy_within(
                    (cursor_a + 1) as usize..(cursor_a + 1) as usize + count,
                    (dest + 1) as usize,
                );
                work[dest as usize] = temp[cursor_temp as usize];
            }
        }
    }
}

/// `GallopLeft`: the number of items of `array`, from `base` on, that are less than `key`, found
/// by galloping from `hint`. The array is searched as the items before the key's place are.
fn gallop_left<T: Copy, F: FnMut(T, T) -> f64>(
    compare: &mut F,
    array: &[T],
    key: T,
    base: isize,
    length: isize,
    hint: isize,
) -> isize {
    let mut last_ofs: isize = 0;
    let mut ofs: isize = 1;
    let hint_value = array[(base + hint) as usize];
    if order(compare, hint_value, key) < 0.0 {
        // array[hint] < key: gallop to the right.
        let max_ofs = length - hint;
        while ofs < max_ofs {
            let value = array[(base + hint + ofs) as usize];
            if order(compare, value, key) >= 0.0 {
                break;
            }
            last_ofs = ofs;
            ofs = (ofs << 1) + 1;
        }
        if ofs > max_ofs {
            ofs = max_ofs;
        }
        last_ofs += hint;
        ofs += hint;
    } else {
        // key <= array[hint]: gallop to the left.
        let max_ofs = hint + 1;
        while ofs < max_ofs {
            let value = array[(base + hint - ofs) as usize];
            if order(compare, value, key) < 0.0 {
                break;
            }
            last_ofs = ofs;
            ofs = (ofs << 1) + 1;
        }
        if ofs > max_ofs {
            ofs = max_ofs;
        }
        let tmp = last_ofs;
        last_ofs = hint - ofs;
        ofs = hint - tmp;
    }
    last_ofs += 1;
    while last_ofs < ofs {
        let m = last_ofs + ((ofs - last_ofs) >> 1);
        let value = array[(base + m) as usize];
        if order(compare, value, key) < 0.0 {
            last_ofs = m + 1;
        } else {
            ofs = m;
        }
    }
    ofs
}

/// `GallopRight`: the number of items of `array`, from `base` on, that are not greater than `key`,
/// found by galloping from `hint`.
fn gallop_right<T: Copy, F: FnMut(T, T) -> f64>(
    compare: &mut F,
    array: &[T],
    key: T,
    base: isize,
    length: isize,
    hint: isize,
) -> isize {
    let mut last_ofs: isize = 0;
    let mut ofs: isize = 1;
    let hint_value = array[(base + hint) as usize];
    if order(compare, key, hint_value) < 0.0 {
        // key < array[hint]: gallop to the left.
        let max_ofs = hint + 1;
        while ofs < max_ofs {
            let value = array[(base + hint - ofs) as usize];
            if order(compare, key, value) >= 0.0 {
                break;
            }
            last_ofs = ofs;
            ofs = (ofs << 1) + 1;
        }
        if ofs > max_ofs {
            ofs = max_ofs;
        }
        let tmp = last_ofs;
        last_ofs = hint - ofs;
        ofs = hint - tmp;
    } else {
        // array[hint] <= key: gallop to the right.
        let max_ofs = length - hint;
        while ofs < max_ofs {
            let value = array[(base + hint + ofs) as usize];
            if order(compare, key, value) < 0.0 {
                break;
            }
            last_ofs = ofs;
            ofs = (ofs << 1) + 1;
        }
        if ofs > max_ofs {
            ofs = max_ofs;
        }
        last_ofs += hint;
        ofs += hint;
    }
    last_ofs += 1;
    while last_ofs < ofs {
        let m = last_ofs + ((ofs - last_ofs) >> 1);
        let value = array[(base + m) as usize];
        if order(compare, key, value) < 0.0 {
            ofs = m;
        } else {
            last_ofs = m + 1;
        }
    }
    ofs
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small deterministic generator, so that the tests are the same on every run.
    fn sequence(seed: u64) -> impl FnMut() -> u64 {
        let mut state = seed;
        move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        }
    }

    #[test]
    fn a_consistent_comparator_sorts_stably_like_std() {
        for length in [0_usize, 1, 2, 5, 31, 63, 64, 65, 200, 1000] {
            let mut next = sequence(length as u64 + 7);
            let mut items: Vec<(i64, usize)> = (0..length)
                .map(|index| ((next() % 50) as i64, index))
                .collect();
            let mut expected = items.clone();
            expected.sort_by_key(|item| item.0);
            sort_js(&mut items, |a, b| (a.0 - b.0) as f64);
            assert_eq!(items, expected, "length {length}");
        }
    }

    #[test]
    fn a_comparator_with_nan_keeps_the_order_of_equal_items() {
        // Every comparison is NaN, which V8 reads as equal: the order is kept.
        let mut items = vec![3, 1, 2];
        sort_js(&mut items, |_, _| f64::NAN);
        assert_eq!(items, vec![3, 1, 2]);
    }

    #[test]
    fn a_descending_run_is_reversed() {
        let mut items = vec![9, 7, 5, 1];
        sort_js(&mut items, |a, b| (a - b) as f64);
        assert_eq!(items, vec![1, 5, 7, 9]);
    }

    #[test]
    fn a_long_array_of_runs_is_merged() {
        let mut next = sequence(99);
        let mut items: Vec<u32> = (0..500).map(|_| (next() % 1000) as u32).collect();
        let mut expected = items.clone();
        expected.sort();
        sort_js(&mut items, |a, b| (a as f64) - (b as f64));
        assert_eq!(items, expected);
    }
}
