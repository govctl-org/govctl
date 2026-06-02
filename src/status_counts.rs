use std::collections::HashMap;
use std::hash::Hash;

pub(crate) type StatusCounts<K> = HashMap<K, usize>;

pub(crate) fn count_by<I, T, K, F>(items: I, mut key: F) -> StatusCounts<K>
where
    I: IntoIterator<Item = T>,
    K: Eq + Hash,
    F: FnMut(T) -> K,
{
    let mut counts = StatusCounts::new();
    for item in items {
        *counts.entry(key(item)).or_insert(0) += 1;
    }
    counts
}

pub(crate) fn count_for<K: Eq + Hash>(counts: &StatusCounts<K>, key: K) -> usize {
    counts.get(&key).copied().unwrap_or(0)
}

pub(crate) fn counts_for_keys<K, const N: usize>(
    counts: &StatusCounts<K>,
    keys: [K; N],
) -> [usize; N]
where
    K: Copy + Eq + Hash,
{
    keys.map(|key| count_for(counts, key))
}

pub(crate) fn total_count<K>(counts: &StatusCounts<K>) -> usize {
    counts.values().sum()
}

#[cfg(test)]
mod tests {
    use super::{count_by, count_for, counts_for_keys, total_count};

    #[test]
    fn count_by_collects_counts_for_keys() {
        let counts = count_by(["draft", "done", "draft"], |status| status);

        assert_eq!(count_for(&counts, "draft"), 2);
        assert_eq!(count_for(&counts, "missing"), 0);
        assert_eq!(total_count(&counts), 3);
    }

    #[test]
    fn counts_for_keys_returns_ordered_counts() {
        let counts = count_by(["active", "queue", "active"], |status| status);

        assert_eq!(
            counts_for_keys(&counts, ["queue", "active", "done"]),
            [1, 2, 0]
        );
    }
}
