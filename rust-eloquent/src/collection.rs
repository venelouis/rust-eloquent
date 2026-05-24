use std::collections::HashMap;
use std::hash::Hash;

/// An extension trait that brings Laravel-style Collection methods natively to Rust's Vec<T>.
pub trait EloquentCollection<T> {
    /// Keys the collection by the given closure's return value
    fn key_by<K, F>(self, f: F) -> HashMap<K, T>
    where
        F: Fn(&T) -> K,
        K: Hash + Eq;

    /// Splits the collection into chunks of the given size
    fn chunk(self, size: usize) -> Vec<Vec<T>>;

    /// Joins the items into a single string using the given separator and closure
    fn implode<F>(&self, separator: &str, f: F) -> String
    where
        F: Fn(&T) -> String;

    /// Sums up the values returned by the closure
    fn sum_by<N, F>(&self, f: F) -> N
    where
        F: Fn(&T) -> N,
        N: std::iter::Sum;

    /// Finds the maximum value returned by the closure
    fn max_by_key<K, F>(&self, f: F) -> Option<&T>
    where
        F: Fn(&T) -> K,
        K: Ord;

    /// Finds the minimum value returned by the closure
    fn min_by_key<K, F>(&self, f: F) -> Option<&T>
    where
        F: Fn(&T) -> K,
        K: Ord;
}

impl<T> EloquentCollection<T> for Vec<T> {
    fn key_by<K, F>(self, f: F) -> HashMap<K, T>
    where
        F: Fn(&T) -> K,
        K: Hash + Eq,
    {
        let mut map = HashMap::new();
        for item in self {
            map.insert(f(&item), item);
        }
        map
    }

    fn chunk(self, size: usize) -> Vec<Vec<T>> {
        if size == 0 {
            return vec![self];
        }

        let mut chunks = vec![];
        let mut current_chunk = vec![];

        for item in self {
            current_chunk.push(item);
            if current_chunk.len() == size {
                chunks.push(current_chunk);
                current_chunk = vec![];
            }
        }

        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        chunks
    }

    fn implode<F>(&self, separator: &str, f: F) -> String
    where
        F: Fn(&T) -> String,
    {
        let strings: Vec<String> = self.iter().map(f).collect();
        strings.join(separator)
    }

    fn sum_by<N, F>(&self, f: F) -> N
    where
        F: Fn(&T) -> N,
        N: std::iter::Sum,
    {
        self.iter().map(f).sum()
    }

    fn max_by_key<K, F>(&self, f: F) -> Option<&T>
    where
        F: Fn(&T) -> K,
        K: Ord,
    {
        self.iter().max_by_key(|item| f(*item))
    }

    fn min_by_key<K, F>(&self, f: F) -> Option<&T>
    where
        F: Fn(&T) -> K,
        K: Ord,
    {
        self.iter().min_by_key(|item| f(*item))
    }
}
