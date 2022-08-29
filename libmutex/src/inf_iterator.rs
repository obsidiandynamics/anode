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

#[cfg(test)]
mod tests;