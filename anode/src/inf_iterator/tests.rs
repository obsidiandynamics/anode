use std::ops::Range;
use crate::inf_iterator::RangeCycle;
use super::{BoundedIterator, InfIterator, IntoInfIterator};

pub struct RangeInfIterator {
    range: Range<usize>,
    pos: usize
}

impl InfIterator for RangeInfIterator {
    type Item = usize;

    fn next(&mut self) -> Self::Item {
        let val = self.range.start + self.pos;

        let m = self.range.end - self.range.start - 1;
        if self.pos < m {
            self.pos += 1;
        } else {
            self.pos = 0;
        }

        val
    }
}

impl IntoInfIterator for Range<u64> {
    type Item = u64;
    type IntoInfIter = RangeCycle<u64>;

    fn into_inf_iter(self) -> Self::IntoInfIter {
        RangeCycle::new(self)
    }
}

#[test]
fn mod_cycle_inf() {
    let range = 5..6;
    let mut range_it = range.into_inf_iter();
    assert_eq!(5, range_it.next());
    assert_eq!(5, range_it.next());
    assert_eq!(5, range_it.next());

    let range = 5..7;
    let mut range_it = range.into_inf_iter();
    assert_eq!(5, range_it.next());
    assert_eq!(6, range_it.next());
    assert_eq!(5, range_it.next());
    assert_eq!(6, range_it.next());

    let range = u64::MAX-3..u64::MAX;
    let mut range_it = range.into_inf_iter();
    assert_eq!(u64::MAX - 3, range_it.next());
    assert_eq!(u64::MAX - 2, range_it.next());
    assert_eq!(u64::MAX - 1, range_it.next());
    assert_eq!(u64::MAX - 3, range_it.next());
}

#[test]
fn mod_cycle_bounded() {
    let range = 5..6;
    let mut range_it: BoundedIterator<_> = range.into_inf_iter().into();
    assert_eq!(Some(5), range_it.next());
    assert_eq!(Some(5), range_it.next());
    assert_eq!(Some(5), range_it.next());

    let range = 5..7;
    let mut range_it: BoundedIterator<_> = range.into_inf_iter().into();
    assert_eq!(Some(5), range_it.next());
    assert_eq!(Some(6), range_it.next());
    assert_eq!(Some(5), range_it.next());
    assert_eq!(Some(6), range_it.next());
}