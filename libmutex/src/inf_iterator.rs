use std::ops::Range;

/// An unbounded iterator that never runs out of values.
pub trait InfIterator {
    /// The type of the elements being iterated over.
    type Item;

    /// Advances the iterator, returning the next value.
    fn next(&mut self) -> Self::Item;
}

impl<T, I: InfIterator<Item = T>> From<I> for BoundedIterator<I> {
    fn from(inf: I) -> Self {
        Self(inf)
    }
}

/// Conversion into an [`InfIterator`].
pub trait IntoInfIterator {
    /// The type of the elements being iterated over.
    type Item;

    /// Which kind of iterator are we turning this into?
    type IntoInfIter: InfIterator<Item = Self::Item>;

    /// Creates an iterator from a value.
    fn into_inf_iter(self) -> Self::IntoInfIter;
}

/// Supports conversion from an [`InfIterator`] to a conventional, bounded [`Iterator`].
pub struct BoundedIterator<I: InfIterator>(I);

impl<T, I: InfIterator<Item = T>> Iterator for BoundedIterator<I> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.0.next())
    }
}

pub trait Successor where Self: Sized {
    fn successor(&self) -> Option<Self>;
}

impl Successor for u64 {
    fn successor(&self) -> Option<Self> {
        if *self == u64::MAX {
            None
        } else {
            Some(self + 1)
        }
    }
}

#[derive(Debug)]
pub struct RangeCycle<T> {
    range: Range<T>,
    item: T
}

impl<T> RangeCycle<T> {
    pub fn new(range: Range<T>) -> Self where T: Copy {
        let start = range.start;
        Self::starting_at(range, start)
    }

    pub fn starting_at(range: Range<T>, item: T) -> Self {
        Self {
            range, item
        }
    }
}

impl<T: Successor + Copy + Eq> InfIterator for RangeCycle<T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Self::Item {
        let current = self.item;
        let next = current.successor().unwrap();
        self.item = if next == self.range.end { self.range.start } else { next };
        current
    }
}

#[cfg(test)]
mod tests;