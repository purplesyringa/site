use core::mem::MaybeUninit;
use criterion::{
    BenchmarkId, Criterion, SamplingMode, Throughput, {criterion_group, criterion_main},
};
use fixed_slice_vec::FixedSliceVec;
use std::time::Duration;
use wyrand::WyRand;

// const CUTOFF: usize = 50_000;
const CUTOFF: usize = 200_000;
// const CUTOFF: usize = 1_000_000;

#[inline(never)]
fn fallback(
    elements: impl Iterator<Item = u64> + Clone,
    elements_len: usize,
    key: &mut impl FnMut(u64) -> usize,
    key_bitness: u32,
    callback: &mut impl FnMut(&mut dyn Iterator<Item = u64>),
) {
    let n_groups = 1 << key_bitness;

    let mut counts: Vec<usize> = vec![0; n_groups];
    for element in elements.clone() {
        counts[key(element) & (n_groups - 1)] += 1;
    }

    let mut group_ptrs: Vec<usize> = vec![0; n_groups];
    for i in 1..n_groups {
        group_ptrs[i] = group_ptrs[i - 1] + counts[i - 1];
    }

    let mut buffer = vec![MaybeUninit::uninit(); elements_len];
    for element in elements {
        let group_ptr = &mut group_ptrs[key(element) & ((1 << key_bitness) - 1)];
        buffer[*group_ptr].write(element);
        *group_ptr += 1;
    }

    let mut end_ptr = 0;
    for i in 0..n_groups {
        let start_ptr = end_ptr;
        end_ptr += counts[i];
        if counts[i] > 0 {
            assert_eq!(end_ptr, group_ptrs[i]); // safety check for initialization!
            let group = &buffer[start_ptr..end_ptr];
            let group = unsafe { &*(group as *const [MaybeUninit<u64>] as *const [u64]) };
            callback(&mut group.iter().copied());
        }
    }
}

struct Bucket<'buffer, T> {
    reserved: FixedSliceVec<'buffer, T>,
    overflow: Vec<T>,
}

impl<'buffer, T> Bucket<'buffer, T> {
    fn new(reserved: FixedSliceVec<'buffer, T>) -> Self {
        Self {
            reserved,
            overflow: Vec::new(),
        }
    }

    fn push(&mut self, element: T) {
        if let Err(element) = self.reserved.try_push(element) {
            self.overflow.push(element.0);
        }
    }

    fn len(&self) -> usize {
        self.reserved.len() + self.overflow.len()
    }

    fn iter(&self) -> core::iter::Chain<core::slice::Iter<T>, core::slice::Iter<T>> {
        self.reserved.iter().chain(self.overflow.iter())
    }
}

pub fn radix_sort(
    elements: impl Iterator<Item = u64> + Clone,
    elements_len: usize,
    key: &mut impl FnMut(u64) -> usize,
    key_bitness: u32,
    callback: &mut impl FnMut(&mut dyn Iterator<Item = u64>),
) {
    // The step at which `key` is consumed. `2 ** BITS` buckets are allocated.
    const BITS: u32 = 8;

    if elements_len <= CUTOFF || key_bitness <= BITS {
        fallback(elements, elements_len, key, key_bitness, callback);
        return;
    }

    let shift = key_bitness - BITS;

    let reserved_capacity = (elements_len >> BITS).max(1); // 0 breaks `chunks_mut`

    // Partitioning a single allocation is more efficient than allocating multiple times
    let mut buffer = vec![MaybeUninit::uninit(); reserved_capacity << BITS];
    let mut reserved = buffer.chunks_mut(reserved_capacity);
    let mut buckets: [Bucket<u64>; 1 << BITS] = core::array::from_fn(|_| {
        Bucket::new(FixedSliceVec::new(reserved.next().unwrap_or(&mut [])))
    });

    for element in elements {
        buckets[(key(element) >> shift) & ((1 << BITS) - 1)].push(element);
    }

    for bucket in buckets {
        radix_sort(
            bucket.iter().copied(),
            bucket.len(),
            key,
            key_bitness - BITS,
            callback,
        );
    }
}

macro_rules! run {
    ($fn:ident, $input:expr, $n:expr, $m:expr) => {{
        let mut total = 0;
        $fn(
            $input,
            $n,
            &mut |element| (element.wrapping_mul(0x9a08c0ebcf5bc11b) >> (64 - $m)) as usize,
            $m,
            &mut |group| {
                total += group.min().unwrap();
            },
        );
        total
    }};
}

fn bench_grouping(c: &mut Criterion) {
    let mut group = c.benchmark_group("grouping");
    group
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(1))
        .sampling_mode(SamplingMode::Flat);
    for shift in 0..10 {
        let n = 80000usize << shift;
        let m = 13 + shift;

        let mut rng = WyRand::new(0x9a08c0ebcf5bc11b);
        let input = (0..n).map(move |_| rng.rand());

        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::new("old", n), &m, |b, &m| {
            b.iter(|| run!(fallback, input.clone(), n, m));
        });
        group.bench_with_input(BenchmarkId::new("new", n), &m, |b, &m| {
            b.iter(|| run!(radix_sort, input.clone(), n, m));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_grouping);
criterion_main!(benches);
