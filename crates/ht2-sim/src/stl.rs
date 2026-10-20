//! The `std::sort` the original is built with (the SGI introsort of its STL, e.g. `0x4143c0`):
//! elements that compare equal end up in the same order as in the original only with the same
//! algorithm.

const THRESHOLD: usize = 16;

/// `std::sort(v.begin(), v.end(), less)`.
pub fn sort<T: Clone>(v: &mut [T], less: impl Fn(&T, &T) -> bool + Copy) {
    if v.is_empty() {
        return;
    }
    let mut depth = 0;
    let mut n = v.len();
    while n != 1 {
        depth += 1;
        n >>= 1;
    }
    introsort_loop(v, 0, v.len(), depth * 2, less);
    final_insertion_sort(v, less);
}

fn median<'a, T>(a: &'a T, b: &'a T, c: &'a T, less: impl Fn(&T, &T) -> bool) -> &'a T {
    if less(a, b) {
        if less(b, c) {
            b
        } else if less(a, c) {
            c
        } else {
            a
        }
    } else if less(a, c) {
        a
    } else if less(b, c) {
        c
    } else {
        b
    }
}

fn introsort_loop<T: Clone>(v: &mut [T], first: usize, mut last: usize, mut depth: usize, less: impl Fn(&T, &T) -> bool + Copy) {
    while last - first > THRESHOLD {
        if depth == 0 {
            partial_sort(&mut v[first..last], less);
            return;
        }
        depth -= 1;
        let pivot = median(&v[first], &v[first + (last - first) / 2], &v[last - 1], less).clone();
        let cut = unguarded_partition(v, first, last, &pivot, less);
        introsort_loop(v, cut, last, depth, less);
        last = cut;
    }
}

fn unguarded_partition<T>(v: &mut [T], mut first: usize, mut last: usize, pivot: &T, less: impl Fn(&T, &T) -> bool) -> usize {
    loop {
        while less(&v[first], pivot) {
            first += 1;
        }
        last -= 1;
        while less(pivot, &v[last]) {
            last -= 1;
        }
        if first >= last {
            return first;
        }
        v.swap(first, last);
        first += 1;
    }
}

fn final_insertion_sort<T: Clone>(v: &mut [T], less: impl Fn(&T, &T) -> bool + Copy) {
    if v.len() > THRESHOLD {
        insertion_sort(&mut v[..THRESHOLD], less);
        for i in THRESHOLD..v.len() {
            let value = v[i].clone();
            unguarded_linear_insert(v, i, value, less);
        }
    } else {
        insertion_sort(v, less);
    }
}

fn insertion_sort<T: Clone>(v: &mut [T], less: impl Fn(&T, &T) -> bool + Copy) {
    for i in 1..v.len() {
        let value = v[i].clone();
        if less(&value, &v[0]) {
            v[..=i].rotate_right(1);
        } else {
            unguarded_linear_insert(v, i, value, less);
        }
    }
}

fn unguarded_linear_insert<T: Clone>(v: &mut [T], mut last: usize, value: T, less: impl Fn(&T, &T) -> bool) {
    while less(&value, &v[last - 1]) {
        v[last] = v[last - 1].clone();
        last -= 1;
    }
    v[last] = value;
}

/// `partial_sort(first, last, last)`: heap sort.
fn partial_sort<T: Clone>(v: &mut [T], less: impl Fn(&T, &T) -> bool + Copy) {
    let len = v.len();
    if len >= 2 {
        let mut parent = (len - 2) / 2;
        loop {
            let value = v[parent].clone();
            adjust_heap(v, parent, len, value, less);
            if parent == 0 {
                break;
            }
            parent -= 1;
        }
    }
    let mut last = len;
    while last > 1 {
        let value = v[last - 1].clone();
        v[last - 1] = v[0].clone();
        adjust_heap(v, 0, last - 1, value, less);
        last -= 1;
    }
}

fn adjust_heap<T: Clone>(v: &mut [T], mut hole: usize, len: usize, value: T, less: impl Fn(&T, &T) -> bool) {
    let top = hole;
    let mut child = 2 * hole + 2;
    while child < len {
        if less(&v[child], &v[child - 1]) {
            child -= 1;
        }
        v[hole] = v[child].clone();
        hole = child;
        child = 2 * (child + 1);
    }
    if child == len {
        v[hole] = v[child - 1].clone();
        hole = child - 1;
    }
    let mut parent = hole.wrapping_sub(1) / 2;
    while hole > top && less(&v[parent], &value) {
        v[hole] = v[parent].clone();
        hole = parent;
        parent = hole.wrapping_sub(1) / 2;
    }
    v[hole] = value;
}

#[cfg(test)]
mod tests {
    #[test]
    fn sorts() {
        let mut s = 12345u32;
        for n in [0usize, 1, 2, 5, 16, 17, 40, 300] {
            let mut v: Vec<(u32, usize)> = (0..n)
                .map(|i| {
                    s = s.wrapping_mul(1103515245).wrapping_add(12345);
                    ((s >> 16) % 7, i)
                })
                .collect();
            super::sort(&mut v, |a, b| a.0 < b.0);
            assert!(v.windows(2).all(|w| w[0].0 <= w[1].0));
            let mut h = v.clone();
            super::partial_sort(&mut h, |a, b| a.0 < b.0);
            assert!(h.windows(2).all(|w| w[0].0 <= w[1].0));
        }
    }
}
