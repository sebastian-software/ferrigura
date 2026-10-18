//! The JavaScript semantics the swimlane layout depends on, as V8 gives them, bit for bit:
//!
//! - [`to_fixed`]: `Number.prototype.toFixed`, by the specification (ECMA-262), with the decision taken on
//!   the exact binary value and ties rounded up in magnitude.
//! - [`js_round`]: `Math.round`, half toward +infinity, with V8's sign of zero.
//! - [`locale_compare`] and [`try_locale_compare`]: `a.localeCompare(b)` in V8 with the default locale (en-US),
//!   for printable ASCII. It is ICU's root collation at tertiary strength, written as a table of the 95
//!   characters in ICU order.
//! - [`js_sort_by`]: `Array.prototype.sort(compare)` as V8 in Chromium 151 runs it, with the same comparisons in
//!   the same order, so that a comparator that is not a total order (a threshold, or a NaN key) gives the
//!   reference's order and does not panic. Exact from 17 to 129 elements; its doc states the gap. The layout's
//!   numeric sorts use it.
//!
//! `Math.hypot` is not here: the port calls `ferrigura_jsmath::hypot`.
//!
//! The data files in `tests/data` hold V8's results for these functions (Node v24.21.0, ICU 78.3), written by
//! `tools/js-semantics.ts`; `tests/js_semantics.rs` checks them.

use std::cmp::Ordering;
use std::marker::PhantomData;

use crate::Error;

/// The most fraction digits `toFixed` takes. JavaScript throws a `RangeError` above it.
const MAX_DIGITS: u32 = 100;

/// The number of fraction digits `format!` prints for a double without rounding. A double is m * 2^-k with
/// k at most 1074, so its decimal expansion has at most 1074 fraction digits; 1100 leaves room.
const EXACT_DIGITS: usize = 1100;

/// `2^52`: every double at or above it is an integer, so `Math.round` returns it unchanged.
const TWO_POW_52: f64 = 4_503_599_627_370_496.0;

/// `Number.prototype.toFixed(digits)`: the decimal string of `x` with `digits` fraction digits.
///
/// - `NaN` gives `"NaN"`, and the infinities give `"Infinity"` and `"-Infinity"`.
/// - For `|x| >= 1e21` the result is `Number::toString` of `x` in exponent form, as the specification says
///   (`"1e+21"`, `"-1.2345e+25"`). Its digits are the shortest that round-trip, which is what Rust's `{:e}`
///   writes, so there is no error case.
/// - Otherwise the sign is `-` when `x < 0`, so `-0` gives `"0.0"` but `-0.04` gives `"-0.0"`, and the number
///   is `n / 10^digits` for the integer `n` nearest to `|x|`. A tie goes to the larger `n`, so the rounding is
///   half up in magnitude: `(0.25).toFixed(1)` is `"0.3"` and `(-2.5).toFixed(0)` is `"-3"`. The decision uses
///   the exact binary value: `format!("{:.1100}")` writes the exact expansion of the double, and the first digit
///   after the last one kept decides (5 or more rounds up). The rounding of `format!` itself (ties to even) is
///   not used for the decision.
///
/// # Panics
///
/// Panics if `digits > 100`, which JavaScript rejects with a `RangeError`.
#[must_use]
pub fn to_fixed(x: f64, digits: u32) -> String {
    assert!(
        digits <= MAX_DIGITS,
        "toFixed takes at most {MAX_DIGITS} fraction digits, not {digits}"
    );
    if x.is_nan() {
        return "NaN".to_string();
    }
    if x.is_infinite() {
        return if x < 0.0 { "-Infinity" } else { "Infinity" }.to_string();
    }
    let sign = if x < 0.0 { "-" } else { "" };
    let magnitude = x.abs();
    if magnitude >= 1e21 {
        return format!("{sign}{}", exponent_form(magnitude));
    }

    let digits = digits as usize;
    let exact = format!("{:.*}", EXACT_DIGITS, magnitude);
    let (integer, fraction) = exact
        .split_once('.')
        .expect("a fixed-point string has a fraction");
    // The integer digits, then the fraction digits that are kept; the next digit decides the rounding.
    let mut kept: Vec<u8> = integer
        .bytes()
        .chain(fraction.bytes().take(digits))
        .collect();
    if fraction.as_bytes()[digits] >= b'5' {
        increment_decimal(&mut kept);
    }

    let split = kept.len() - digits;
    let mut out = String::with_capacity(sign.len() + kept.len() + 1);
    out.push_str(sign);
    out.push_str(ascii(&kept[..split]));
    if digits > 0 {
        out.push('.');
        out.push_str(ascii(&kept[split..]));
    }
    out
}

/// `Number::toString` of a finite positive double of at least 1e21: the shortest round-trip digits, in the
/// form `d.ddde+N`. A digit string of one digit has no point.
fn exponent_form(magnitude: f64) -> String {
    let scientific = format!("{magnitude:e}");
    let (mantissa, exponent) = scientific
        .split_once('e')
        .expect("an exponent form has an e");
    format!("{mantissa}e+{exponent}")
}

/// Adds one to a string of decimal digits, carrying; a carry out of the first digit adds a leading `1`.
fn increment_decimal(digits: &mut Vec<u8>) {
    for digit in digits.iter_mut().rev() {
        if *digit == b'9' {
            *digit = b'0';
        } else {
            *digit += 1;
            return;
        }
    }
    digits.insert(0, b'1');
}

/// The bytes of a decimal string as text; they are ASCII digits, so this cannot fail.
fn ascii(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).expect("decimal digits are ASCII")
}

/// `Math.round(x)`: the integer nearest to `x`, with a tie going toward +infinity (`Math.round(-2.5)` is
/// `-2`, `Math.round(2.5)` is `3`).
///
/// `NaN`, the infinities, the zeros, and every value with `|x| >= 2^52` (already an integer) come back as they
/// are. A result of zero keeps the sign V8 gives it: a negative `x` that rounds to zero gives `-0`
/// (`Math.round(-0.4)` is `-0`), and a positive one gives `+0`.
#[must_use]
pub fn js_round(x: f64) -> f64 {
    if !x.is_finite() || x == 0.0 || x.abs() >= TWO_POW_52 {
        return x;
    }
    let floor = x.floor();
    // The difference `x - floor` decides the tie, and it is exact in the cases that matter. For |x| >= 1 it is
    // a multiple of the ulp of `x` below 1, so representable. For -1 <= x <= -0.5 it is `x + 1`, exact by
    // Sterbenz's lemma. For -0.5 < x < 0 the exact difference is above 0.5, and rounding cannot take it below
    // 0.5, so the result is zero, as it is for the exact value.
    let rounded = if x - floor >= 0.5 { floor + 1.0 } else { floor };
    if rounded == 0.0 {
        if x < 0.0 { -0.0 } else { 0.0 }
    } else {
        rounded
    }
}

/// The printable ASCII characters, 0x20 to 0x7E, in the order ICU's root collation sorts them, as V8's
/// `localeCompare` does with the default locale. The order was read from node (v24.21.0, ICU 78.3) by
/// sorting the 95 characters with `localeCompare`. Letters appear as lowercase then uppercase pairs: the two
/// cases share a primary weight, and the lowercase form sorts first at the tertiary level.
const ICU_ORDER: &[u8] = b" _-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$0123456789aAbBcCdDeEfFgGhHiIjJkKlLmMnNoOpPqQrRsStTuUvVwWxXyYzZ";

/// The primary weight of each byte: its position in [`ICU_ORDER`], with an uppercase letter sharing the
/// weight of its lowercase form. The entries for bytes outside printable ASCII are never read.
const PRIMARY: [u8; 128] = primary_table();

const fn primary_table() -> [u8; 128] {
    let mut table = [0u8; 128];
    let mut next = 0u8;
    let mut i = 0;
    while i < ICU_ORDER.len() {
        let byte = ICU_ORDER[i];
        if byte.is_ascii_uppercase() {
            table[byte as usize] = table[byte.to_ascii_lowercase() as usize];
        } else {
            table[byte as usize] = next;
            next += 1;
        }
        i += 1;
    }
    table
}

/// Whether a byte is printable ASCII, 0x20 to 0x7E.
const fn is_printable_ascii(byte: u8) -> bool {
    matches!(byte, 0x20..=0x7e)
}

/// `a.localeCompare(b)` in V8 with the default locale (en-US), for printable ASCII strings.
///
/// The order is ICU's root collation at tertiary strength with non-ignorable punctuation: the primary weights
/// are compared over the whole strings first (a string that is a prefix of the other comes first), then, when
/// the primary weights are equal, the case of the letters, lowercase first at the first difference. Two
/// different printable ASCII strings never compare `Equal`. The order is checked against V8 in
/// `tests/js_semantics.rs`, exhaustively for strings of up to two characters.
///
/// # Panics
///
/// Panics if either string has a byte outside printable ASCII (a control character, or any non-ASCII text).
/// [`try_locale_compare`] returns an error instead.
#[must_use]
pub fn locale_compare(a: &str, b: &str) -> Ordering {
    match try_locale_compare(a, b) {
        Ok(ordering) => ordering,
        Err(error) => panic!("localeCompare: {error}"),
    }
}

/// [`locale_compare`], for input that is not known to be printable ASCII.
///
/// # Errors
///
/// Returns [`Error::NotPrintableAscii`] with the string that has a byte outside printable ASCII (0x20 to
/// 0x7E). The collation is not ported for those.
pub fn try_locale_compare(a: &str, b: &str) -> Result<Ordering, Error> {
    for text in [a, b] {
        if !text.bytes().all(is_printable_ascii) {
            return Err(Error::NotPrintableAscii(text.to_string()));
        }
    }
    let (a, b) = (a.as_bytes(), b.as_bytes());

    // The primary level decides over the whole strings. Each ASCII byte has one primary weight.
    for (x, y) in a.iter().zip(b) {
        match PRIMARY[usize::from(*x)].cmp(&PRIMARY[usize::from(*y)]) {
            Ordering::Equal => {}
            other => return Ok(other),
        }
    }
    match a.len().cmp(&b.len()) {
        Ordering::Equal => {}
        other => return Ok(other),
    }

    // Equal primary weights mean the same characters, up to case. The tertiary level takes the first
    // difference in case; a lowercase letter (false) sorts before an uppercase one (true).
    for (x, y) in a.iter().zip(b) {
        match x.is_ascii_uppercase().cmp(&y.is_ascii_uppercase()) {
            Ordering::Equal => {}
            other => return Ok(other),
        }
    }
    Ok(Ordering::Equal)
}

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

/// Arrays shorter than this are sorted by a binary insertion from the second element, with no run (Chromium 151).
const SHORT_ARRAY: usize = 16;

/// The longest array sorted by the rules of Chromium 151; see [`js_sort_by`].
const CHROMIUM_EXACT_MAX: usize = 129;

/// The sort state of one call (`SortState`): the comparator, the pending runs and `minGallop`, and whether the
/// rules of Chromium 151 apply (arrays of up to [`CHROMIUM_EXACT_MAX`] elements) or those of V8 13.6.
struct Sorter<T, F> {
    compare: F,
    runs: Vec<Run>,
    min_gallop: usize,
    chromium: bool,
    marker: PhantomData<T>,
}

/// `Array.prototype.sort(compare)` on `items`, as V8 in Chromium 151 runs it, the browser of the reference.
/// `compare` returns the number that the JavaScript comparator returns, and a NaN counts as `+0`, as V8 converts
/// it. A negative result puts the first argument first. The comparators of the layout are not always total orders
/// (a threshold on a difference, or a NaN key): `slice::sort_by` may panic on them, and the order then depends on
/// the comparisons the sort makes, which this one makes as Chromium does, in the same order.
///
/// The sort is TimSort (`src/builtins/array-sort.tq`, a port of CPython's `listsort`). Chromium 151 differs from
/// V8 13.6 (Node 24) in four places, found from its comparator calls (`tests/js_sort.rs`):
///
/// - Fewer than 16 elements: a binary insertion sort from the second element, with no run first (at the first
///   call of a function; see the gap).
/// - 64 elements and more: the minimum run length is `n` shifted right until it is below 64, not rounded up.
/// - `MergeCollapse`: the invariant is `run[n - 2] >= run[n - 1] + run[n]`, a tie of `run[n - 1]` and
///   `run[n + 1]` merges the lower pair, and the top two runs merge only if the lower one is strictly shorter.
/// - A merge with a second run of one element is a binary search for it in the first run, then a rotation.
///
/// From 16 to 63 elements both sort alike. The sort is Chromium's exactly from 17 to 129 elements, at every call
/// site. The gap has two parts, and the ignored tests in `tests/js_sort.rs` show both:
///
/// - Up to 16 elements Chromium's comparisons depend on the JIT state of the code that calls `sort`. The first
///   call of a function sorts as above, but a call site that has run a few thousand sorts, or code made by
///   `new Function`, makes other comparisons, and may repeat the first one. No single order is the reference's
///   there; this sort takes the first call's.
/// - From 130 elements on, Chromium sorts by rules that are not reproduced. This sort keeps V8 13.6's TimSort
///   there: its call sequence differs from Chromium's, and so does its permutation for an inconsistent
///   comparator.
///
/// For a consistent comparator every stable sort gives the same permutation, so the gap shows only in the
/// comparator calls and with an inconsistent comparator.
pub fn js_sort_by<T: Copy>(items: &mut [T], compare: impl Fn(T, T) -> f64) {
    let length = items.len();
    if length < 2 {
        return;
    }
    let mut sorter = Sorter {
        compare,
        runs: Vec::new(),
        min_gallop: MIN_GALLOP_WINS,
        chromium: length <= CHROMIUM_EXACT_MAX,
        marker: PhantomData,
    };
    if length < SHORT_ARRAY {
        sorter.binary_insertion_sort(items, 0, 1, length);
    } else {
        sorter.sort(items);
    }
}

/// The minimum run length of Chromium 151: `n` shifted right until it is below 64, with no rounding up.
fn chromium_min_run_length(mut n: usize) -> usize {
    while n >= 64 {
        n >>= 1;
    }
    n
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

impl<T: Copy, F: Fn(T, T) -> f64> Sorter<T, F> {
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
        let min_run = if self.chromium {
            chromium_min_run_length(remaining)
        } else {
            compute_min_run_length(remaining)
        };
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

    /// `RunInvariantEstablished`: the run `n - 2` on the stack is longer than the sum of the two runs above it
    /// (at least as long, in Chromium).
    fn run_invariant_established(&self, n: usize) -> bool {
        if n < 2 {
            return true;
        }
        let below = self.runs[n - 2].length;
        let above = self.runs[n - 1].length + self.runs[n].length;
        if self.chromium {
            below >= above
        } else {
            below > above
        }
    }

    /// `MergeCollapse`: merges the pending runs until the stack keeps its invariant. In Chromium a tie of the runs
    /// around the top pair merges the lower pair, and the top pair merges only if its lower run is shorter.
    fn merge_collapse(&mut self, work: &mut [T]) {
        while self.runs.len() > 1 {
            let mut n = self.runs.len() - 2;
            if !self.run_invariant_established(n + 1) || !self.run_invariant_established(n) {
                let (lower, upper) = (self.runs[n - 1].length, self.runs[n + 1].length);
                if lower < upper || (self.chromium && lower == upper) {
                    n -= 1;
                }
                self.merge_at(work, n);
            } else {
                let (lower, upper) = (self.runs[n].length, self.runs[n + 1].length);
                if lower < upper || (!self.chromium && lower == upper) {
                    self.merge_at(work, n);
                } else {
                    break;
                }
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
        if self.chromium && length_b == 1 {
            // Chromium: a run of one element goes after the elements of the first run that it does not precede,
            // found by a binary search, with no galloping.
            let (mut left, mut right) = (0, length_a);
            while left < right {
                let mid = left + ((right - left) >> 1);
                if self.order(key_right, work[base_a + mid]) < 0.0 {
                    right = mid;
                } else {
                    left = mid + 1;
                }
            }
            work[base_a + left..=base_b].rotate_right(1);
            return;
        }
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
        js_sort_by(&mut items, |a, b| f64::from(a.0) - f64::from(b.0));
        assert_eq!(items, expected);
    }

    #[test]
    fn a_comparator_that_is_nan_is_equal() {
        let mut items = vec![3.0, 1.0, 2.0];
        js_sort_by(&mut items, |a, b| (a - b) * f64::NAN);
        assert_eq!(items, vec![3.0, 1.0, 2.0]);
    }

    #[test]
    fn short_runs_and_descending_runs_are_sorted() {
        let mut items = vec![5, 4, 3, 2, 1];
        js_sort_by(&mut items, |a, b| f64::from(a - b));
        assert_eq!(items, vec![1, 2, 3, 4, 5]);
        let mut items = vec![2, 1];
        js_sort_by(&mut items, |a, b| f64::from(a - b));
        assert_eq!(items, vec![1, 2]);
    }
    /// The indices of `keys`, sorted with the comparator `|a - b| > 1 ? a - b : index order`, which is not a total
    /// order. Each expected order below is Chromium 151's for the same comparator (node 24 gives the same here).
    fn sorted_by_threshold(keys: &[f64]) -> Vec<usize> {
        let mut items: Vec<(f64, usize)> = keys.iter().enumerate().map(|(i, &k)| (k, i)).collect();
        js_sort_by(&mut items, |a, b| {
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

    fn fixed(x: f64, digits: u32) -> String {
        to_fixed(x, digits)
    }

    #[test]
    fn to_fixed_ties_round_up_in_magnitude() {
        assert_eq!(fixed(0.25, 1), "0.3");
        assert_eq!(fixed(0.125, 2), "0.13");
        assert_eq!(fixed(2.5, 0), "3");
        assert_eq!(fixed(-2.5, 0), "-3");
        assert_eq!(fixed(0.5, 0), "1");
        assert_eq!(fixed(1.5, 0), "2");
        assert_eq!(fixed(-0.5, 0), "-1");
    }

    #[test]
    fn to_fixed_decides_on_the_exact_binary_value() {
        // 0.35 is 0.34999999999999997..., below the tie, so it rounds down.
        assert_eq!(fixed(0.35, 1), "0.3");
        // 1.005 is 1.00499999999999989..., and 1.45 is 1.4499999999999999556...
        assert_eq!(fixed(1.005, 2), "1.00");
        assert_eq!(fixed(1.45, 1), "1.4");
        // 8.345 is 8.3450000000000006394..., above the tie.
        assert_eq!(fixed(8.345, 2), "8.35");
    }

    #[test]
    fn to_fixed_keeps_the_sign_of_a_value_that_rounds_to_zero() {
        assert_eq!(fixed(-0.04, 1), "-0.0");
        assert_eq!(fixed(-1e-300, 2), "-0.00");
        assert_eq!(fixed(-0.0, 1), "0.0");
        assert_eq!(fixed(0.0, 0), "0");
        assert_eq!(fixed(1e-7, 6), "0.000000");
        assert_eq!(fixed(5e-324, 0), "0");
    }

    #[test]
    fn to_fixed_carries_into_a_new_digit() {
        assert_eq!(fixed(9.96, 1), "10.0");
        assert_eq!(fixed(99.5, 0), "100");
        assert_eq!(fixed(0.999, 2), "1.00");
    }

    #[test]
    fn to_fixed_special_values_and_large_magnitudes() {
        assert_eq!(fixed(f64::NAN, 2), "NaN");
        assert_eq!(fixed(f64::INFINITY, 2), "Infinity");
        assert_eq!(fixed(f64::NEG_INFINITY, 2), "-Infinity");
        assert_eq!(fixed(1e20, 0), "100000000000000000000");
        assert_eq!(fixed(1e21, 2), "1e+21");
        assert_eq!(fixed(-1e21, 2), "-1e+21");
        assert_eq!(fixed(1.2345e25, 3), "1.2345e+25");
        assert_eq!(fixed(f64::MAX, 0), "1.7976931348623157e+308");
    }

    #[test]
    fn to_fixed_hundred_digits_are_exact() {
        // The exact expansion of 0.1 has 55 significant digits, then zeros.
        let text = fixed(0.1, 100);
        assert_eq!(
            text,
            format!(
                "0.1000000000000000055511151231257827021181583404541015625{}",
                "0".repeat(45)
            )
        );
        // The smallest subnormal has the longest expansion: 1074 fraction digits, the last a 5.
        let smallest = format!("{:.*}", EXACT_DIGITS, f64::from_bits(1));
        let fraction = smallest.split_once('.').expect("fraction").1;
        assert_eq!(fraction.trim_end_matches('0').len(), 1074);
        assert!(fraction.trim_end_matches('0').ends_with('5'));
    }

    #[test]
    #[should_panic(expected = "at most 100")]
    fn to_fixed_rejects_more_than_100_digits() {
        let _ = to_fixed(1.0, 101);
    }

    #[test]
    fn js_round_ties_go_toward_positive_infinity() {
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(-2.5), -2.0);
        assert_eq!(js_round(1.5), 2.0);
        assert_eq!(js_round(-1.5), -1.0);
        assert_eq!(js_round(4_503_599_627_370_495.5), 4_503_599_627_370_496.0);
        assert_eq!(js_round(-4_503_599_627_370_495.5), -4_503_599_627_370_495.0);
    }

    #[test]
    fn js_round_keeps_the_sign_of_zero() {
        assert_eq!(js_round(-0.0).to_bits(), (-0.0f64).to_bits());
        assert_eq!(js_round(0.0).to_bits(), 0.0f64.to_bits());
        assert_eq!(js_round(-0.5).to_bits(), (-0.0f64).to_bits());
        assert_eq!(js_round(-0.4).to_bits(), (-0.0f64).to_bits());
        assert_eq!(js_round(0.4).to_bits(), 0.0f64.to_bits());
        assert_eq!(
            js_round(-0.49999999999999994).to_bits(),
            (-0.0f64).to_bits()
        );
        assert_eq!(js_round(0.49999999999999994).to_bits(), 0.0f64.to_bits());
        assert_eq!(js_round(-1e-320).to_bits(), (-0.0f64).to_bits());
    }

    #[test]
    fn js_round_leaves_integers_and_special_values_alone() {
        let big = 2f64.powi(52) + 1.0;
        assert_eq!(js_round(big), big);
        assert_eq!(js_round(-2f64.powi(52)), -2f64.powi(52));
        assert_eq!(js_round(f64::INFINITY), f64::INFINITY);
        assert_eq!(js_round(f64::NEG_INFINITY), f64::NEG_INFINITY);
        assert!(js_round(f64::NAN).is_nan());
        assert_eq!(js_round(-0.5000000000000001), -1.0);
    }

    fn sorted(mut words: Vec<&str>) -> Vec<&str> {
        words.sort_by(|a, b| locale_compare(a, b));
        words
    }

    #[test]
    fn locale_compare_orders_like_v8() {
        let expected = [
            "_x", "10", "9", "a", "A", "a_b", "a-b", "ab", "b", "B", "z", "Z",
        ];
        let mut shuffled = expected.to_vec();
        shuffled.reverse();
        assert_eq!(sorted(shuffled), expected);
        assert_eq!(locale_compare("a_b", "a-b"), Ordering::Less);
        assert_eq!(locale_compare("A", "a_b"), Ordering::Less);
        assert_eq!(locale_compare("", " "), Ordering::Less);
        assert_eq!(locale_compare("zZ", "Zz"), Ordering::Less);
        assert_eq!(locale_compare("ZZ", "Zz"), Ordering::Greater);
    }

    #[test]
    fn locale_compare_equal_strings_are_equal() {
        assert_eq!(locale_compare("", ""), Ordering::Equal);
        assert_eq!(locale_compare("Node-7_x", "Node-7_x"), Ordering::Equal);
    }

    #[test]
    fn locale_compare_punctuation_precedes_digits_and_letters() {
        let order = " _-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$0123456789aAbBcCdDeEfFgGhHiIjJkKlLmMnNoOpPqQrRsStTuUvVwWxXyYzZ";
        let chars: Vec<String> = order.chars().map(String::from).collect();
        let mut sorted_chars = chars.clone();
        sorted_chars.sort_by(|a, b| locale_compare(a, b));
        assert_eq!(sorted_chars, chars);
    }

    #[test]
    fn try_locale_compare_refuses_what_is_not_printable_ascii() {
        assert_eq!(
            try_locale_compare("a\tb", "a"),
            Err(Error::NotPrintableAscii("a\tb".to_string()))
        );
        assert_eq!(
            try_locale_compare("a", "é"),
            Err(Error::NotPrintableAscii("é".to_string()))
        );
        assert_eq!(try_locale_compare("a", "b"), Ok(Ordering::Less));
    }

    #[test]
    #[should_panic(expected = "localeCompare")]
    fn locale_compare_panics_outside_printable_ascii() {
        let _ = locale_compare("\n", "a");
    }
}
