//! One non-generic stable sort for the whole workspace.
//!
//! `slice::sort_by` and its relatives are generic: the compiler copies the whole merge sort for
//! every element type and comparator closure, and the WASM carried about 60 of those copies
//! (`docs/decisions/2026-11-21-wasm-built-for-size.md`). Here the sorting itself works on a
//! permutation of `usize` indices through a `&mut dyn FnMut(usize, usize) -> Ordering`, so there
//! is one copy of the algorithm; the generic wrappers only build the comparator and apply the
//! permutation with `slice::swap`.
//!
//! Every function is stable: elements that compare equal keep their input order. A call that
//! used `sort_unstable*` may use these as well, because a stable order is one valid unstable
//! order.

#![forbid(unsafe_code)]

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

/// Runs up to this length are sorted by insertion; longer ones are merged.
const RUN: usize = 12;

/// Sorts `v` by `cmp`, keeping the input order of equal elements.
pub fn sort_by<T>(v: &mut [T], mut cmp: impl FnMut(&T, &T) -> Ordering) {
    if v.len() < 2 {
        return;
    }
    let mut order = identity(v.len());
    sort_indices(&mut order, &mut |a, b| cmp(&v[a], &v[b]));
    apply_order(&mut order, &mut |a, b| v.swap(a, b));
}

/// Sorts `v` by the key that `key` computes for each comparison. Use [`sort_by_cached_key`]
/// when the key is expensive.
pub fn sort_by_key<T, K: Ord>(v: &mut [T], mut key: impl FnMut(&T) -> K) {
    sort_by(v, |a, b| key(a).cmp(&key(b)));
}

/// Sorts `v` by a key that `key` computes once per element.
pub fn sort_by_cached_key<T, K: Ord>(v: &mut [T], key: impl FnMut(&T) -> K) {
    if v.len() < 2 {
        return;
    }
    let keys: Vec<K> = v.iter().map(key).collect();
    let mut order = identity(v.len());
    sort_indices(&mut order, &mut |a, b| keys[a].cmp(&keys[b]));
    apply_order(&mut order, &mut |a, b| v.swap(a, b));
}

/// Sorts `v` in ascending order.
pub fn sort<T: Ord>(v: &mut [T]) {
    sort_by(v, T::cmp);
}

/// Builds a `BTreeMap` by inserting one pair at a time; a later pair replaces an earlier one
/// with the same key, as `collect()` does.
///
/// `collect()` into a `BTreeMap` (and `BTreeMap::from`) first sorts the pairs with the generic
/// `slice::sort_by`, which the compiler copies per key and value type. Inserting needs no sort
/// (`docs/decisions/2026-11-22-wasm-cuts-without-a-speed-cost.md`).
pub fn map_of<K: Ord, V>(pairs: impl IntoIterator<Item = (K, V)>) -> BTreeMap<K, V> {
    let mut map = BTreeMap::new();
    map.extend(pairs);
    map
}

/// Builds a `BTreeSet` by inserting one value at a time. See [`map_of`] for the reason.
pub fn set_of<T: Ord>(values: impl IntoIterator<Item = T>) -> BTreeSet<T> {
    let mut set = BTreeSet::new();
    set.extend(values);
    set
}

fn identity(len: usize) -> Vec<usize> {
    (0..len).collect()
}

/// Puts the indices `0..order.len()` (given in any order) into the order `cmp` defines; equal
/// indices keep their relative order. This is the only copy of the sorting algorithm.
fn sort_indices(order: &mut [usize], cmp: &mut dyn FnMut(usize, usize) -> Ordering) {
    let len = order.len();
    for run in order.chunks_mut(RUN) {
        for i in 1..run.len() {
            let mut j = i;
            while j > 0 && cmp(run[j - 1], run[j]) == Ordering::Greater {
                run.swap(j - 1, j);
                j -= 1;
            }
        }
    }
    if len <= RUN {
        return;
    }
    // Merge passes, from `order` into `scratch` and back. `in_order` says which holds the data.
    let mut scratch = order.to_vec();
    let mut in_order = true;
    let mut width = RUN;
    while width < len {
        let (src, dst): (&[usize], &mut [usize]) = if in_order {
            (&*order, &mut scratch[..])
        } else {
            (&scratch[..], &mut *order)
        };
        let mut start = 0;
        while start < len {
            let mid = (start + width).min(len);
            let end = (start + 2 * width).min(len);
            merge(&src[start..mid], &src[mid..end], &mut dst[start..end], cmp);
            start = end;
        }
        in_order = !in_order;
        width *= 2;
    }
    if !in_order {
        order.copy_from_slice(&scratch);
    }
}

/// Merges two sorted runs into `out`; on a tie the element of `left` goes first.
fn merge(
    left: &[usize],
    right: &[usize],
    out: &mut [usize],
    cmp: &mut dyn FnMut(usize, usize) -> Ordering,
) {
    let (mut l, mut r, mut o) = (0, 0, 0);
    while l < left.len() && r < right.len() {
        if cmp(left[l], right[r]) == Ordering::Greater {
            out[o] = right[r];
            r += 1;
        } else {
            out[o] = left[l];
            l += 1;
        }
        o += 1;
    }
    out[o..o + left.len() - l].copy_from_slice(&left[l..]);
    o += left.len() - l;
    out[o..].copy_from_slice(&right[r..]);
}

/// Rearranges a sequence with `swap` so that position `i` ends with the element that was at
/// `order[i]`. Uses `order` as scratch.
fn apply_order(order: &mut [usize], swap: &mut dyn FnMut(usize, usize)) {
    for start in 0..order.len() {
        let mut at = start;
        loop {
            let from = order[at];
            order[at] = at;
            if from == start {
                break;
            }
            swap(at, from);
            at = from;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small deterministic generator (xorshift64*), so the tests need no dependency.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }

        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
    }

    /// Lengths that straddle the insertion run, one merge, and several merge passes.
    const LENGTHS: [usize; 18] = [
        0, 1, 2, 3, 11, 12, 13, 23, 24, 25, 36, 37, 100, 255, 256, 257, 1000, 4097,
    ];

    #[test]
    fn empty_and_single_are_unchanged() {
        let mut none: Vec<u32> = Vec::new();
        sort(&mut none);
        assert!(none.is_empty());
        let mut one = vec![7];
        sort(&mut one);
        assert_eq!(one, [7]);
    }

    #[test]
    fn sorted_input_stays_sorted() {
        for len in LENGTHS {
            let mut v: Vec<usize> = (0..len).collect();
            sort(&mut v);
            assert_eq!(v, (0..len).collect::<Vec<_>>(), "len {len}");
        }
    }

    #[test]
    fn reversed_input_is_sorted() {
        for len in LENGTHS {
            let mut v: Vec<usize> = (0..len).rev().collect();
            sort(&mut v);
            assert_eq!(v, (0..len).collect::<Vec<_>>(), "len {len}");
        }
    }

    #[test]
    fn equal_keys_keep_input_order() {
        for len in LENGTHS {
            // (key, position): only three keys, so the runs of equal keys are long.
            let mut rng = Rng(0x9E37_79B9_7F4A_7C15 ^ len as u64);
            let mut v: Vec<(u64, usize)> = (0..len).map(|i| (rng.below(3), i)).collect();
            sort_by(&mut v, |a, b| a.0.cmp(&b.0));
            for pair in v.windows(2) {
                assert!(pair[0].0 < pair[1].0 || (pair[0].0 == pair[1].0 && pair[0].1 < pair[1].1));
            }
            let mut all_equal: Vec<(u64, usize)> = (0..len).map(|i| (5, i)).collect();
            sort_by_key(&mut all_equal, |p| p.0);
            assert!(
                all_equal.iter().enumerate().all(|(i, p)| p.1 == i),
                "len {len}"
            );
        }
    }

    // The test calls `sort_by` on purpose, to compare it with ours.
    #[allow(clippy::unnecessary_sort_by)]
    #[test]
    fn matches_the_standard_stable_sort() {
        let mut rng = Rng(0xDEAD_BEEF_CAFE_F00D);
        for round in 0..300 {
            let len = (rng.below(300)) as usize;
            let spread = 1 + rng.below(if round % 3 == 0 { 4 } else { 1000 });
            let v: Vec<(u64, u64)> = (0..len as u64).map(|i| (rng.below(spread), i)).collect();

            let mut want = v.clone();
            want.sort_by(|a, b| b.0.cmp(&a.0));
            let mut got = v.clone();
            sort_by(&mut got, |a, b| b.0.cmp(&a.0));
            assert_eq!(got, want, "sort_by, round {round}");

            let mut want = v.clone();
            want.sort_by_key(|p| p.0 % 7);
            let mut got = v.clone();
            sort_by_key(&mut got, |p| p.0 % 7);
            assert_eq!(got, want, "sort_by_key, round {round}");

            let mut want = v.clone();
            want.sort_by_cached_key(|p| format!("{:03}", p.0));
            let mut got = v.clone();
            sort_by_cached_key(&mut got, |p| format!("{:03}", p.0));
            assert_eq!(got, want, "sort_by_cached_key, round {round}");

            let mut want = v.clone();
            want.sort();
            let mut got = v.clone();
            sort(&mut got);
            assert_eq!(got, want, "sort, round {round}");
        }
    }

    #[test]
    fn moves_values_without_cloning() {
        let mut v: Vec<String> = ["pear", "apple", "fig", "banana", "apple"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        sort(&mut v);
        assert_eq!(v, ["apple", "apple", "banana", "fig", "pear"]);
    }

    #[test]
    fn sorts_floats_with_a_total_order() {
        let mut v = vec![2.5, -1.0, 0.0, 9.75, -3.5];
        sort_by(&mut v, f64::total_cmp);
        assert_eq!(v, [-3.5, -1.0, 0.0, 2.5, 9.75]);
    }

    #[test]
    fn cached_key_runs_once_per_element() {
        let mut calls = 0;
        let mut v: Vec<u32> = (0..50).rev().collect();
        sort_by_cached_key(&mut v, |x| {
            calls += 1;
            *x
        });
        assert_eq!(calls, 50);
        assert!(v.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn apply_order_follows_the_permutation() {
        let mut data = vec!['a', 'b', 'c', 'd', 'e'];
        let mut order = vec![2, 0, 1, 4, 3];
        apply_order(&mut order, &mut |a, b| data.swap(a, b));
        assert_eq!(data, ['c', 'a', 'b', 'e', 'd']);
    }
}
