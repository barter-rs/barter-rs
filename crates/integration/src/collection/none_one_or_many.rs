use crate::collection::one_or_many::OneOrMany;
use itertools::Either;
use serde::{Deserialize, Serialize};
use std::{
    borrow::{Borrow, BorrowMut},
    iter::{FromIterator, IntoIterator, once},
};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Deserialize, Serialize)]
pub enum NoneOneOrMany<T> {
    #[default]
    None,
    One(T),
    Many(Vec<T>),
}

// Utility methods
impl<T> NoneOneOrMany<T> {
    pub fn map<U, F>(self, f: F) -> NoneOneOrMany<U>
    where
        F: Fn(T) -> U,
    {
        match self {
            NoneOneOrMany::None => NoneOneOrMany::None,
            NoneOneOrMany::One(x) => NoneOneOrMany::One(f(x)),
            NoneOneOrMany::Many(vec) => NoneOneOrMany::Many(vec.into_iter().map(f).collect()),
        }
    }

    #[must_use]
    pub fn extend<Iter>(self, other: Iter) -> Self
    where
        Iter: IntoIterator<Item = T>,
    {
        let mut other = other.into_iter();

        let Some(first) = other.next() else {
            return self;
        };

        // Chained so the Vec is sized once from the whole sequence: growing a One
        // through into_vec would take a Vec of one and immediately reallocate
        // for the second item.
        Self::Many(match self {
            Self::None => match other.next() {
                None => return Self::One(first),
                Some(second) => once(first).chain(once(second)).chain(other).collect(),
            },
            Self::One(item) => once(item).chain(once(first)).chain(other).collect(),
            Self::Many(mut items) => {
                items.extend(once(first).chain(other));
                items
            }
        })
    }

    pub fn contains(&self, item: &T) -> bool
    where
        T: PartialEq,
    {
        match self {
            Self::None => false,
            Self::One(value) => value == item,
            Self::Many(values) => values.contains(item),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            NoneOneOrMany::None => 0,
            NoneOneOrMany::One(_) => 1,
            NoneOneOrMany::Many(items) => items.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.is_none()
    }

    pub fn is_none(&self) -> bool {
        matches!(self, NoneOneOrMany::None)
    }

    pub fn is_one(&self) -> bool {
        matches!(self, NoneOneOrMany::One(_))
    }

    pub fn is_many(&self) -> bool {
        matches!(self, NoneOneOrMany::Many(_))
    }

    pub fn into_option(self) -> Option<OneOrMany<T>> {
        match self {
            NoneOneOrMany::None => None,
            NoneOneOrMany::One(one) => Some(OneOrMany::One(one)),
            NoneOneOrMany::Many(many) => Some(OneOrMany::Many(many)),
        }
    }

    pub fn into_vec(self) -> Vec<T> {
        match self {
            NoneOneOrMany::None => vec![],
            NoneOneOrMany::One(item) => vec![item],
            NoneOneOrMany::Many(items) => items,
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.as_ref().iter()
    }
}

// AsRef implementation for slice access
impl<T> AsRef<[T]> for NoneOneOrMany<T> {
    fn as_ref(&self) -> &[T] {
        match self {
            Self::None => &[],
            Self::One(item) => std::slice::from_ref(item),
            Self::Many(items) => items.as_slice(),
        }
    }
}

// Borrow implementation for slice access
impl<T> Borrow<[T]> for NoneOneOrMany<T> {
    fn borrow(&self) -> &[T] {
        self.as_ref()
    }
}

// BorrowMut implementation for mutable slice access
impl<T> BorrowMut<[T]> for NoneOneOrMany<T> {
    fn borrow_mut(&mut self) -> &mut [T] {
        match self {
            Self::None => &mut [],
            Self::One(item) => std::slice::from_mut(item),
            Self::Many(items) => items.as_mut_slice(),
        }
    }
}

// Convert from Option into NoneOneOrMany
impl<T> From<Option<T>> for NoneOneOrMany<T> {
    fn from(opt: Option<T>) -> Self {
        match opt {
            None => NoneOneOrMany::None,
            Some(item) => NoneOneOrMany::One(item),
        }
    }
}

// Convert from Vec into NoneOneOrMany
impl<T> From<Vec<T>> for NoneOneOrMany<T> {
    fn from(items: Vec<T>) -> Self {
        match items.len() {
            0 => NoneOneOrMany::None,
            1 => NoneOneOrMany::One(items.into_iter().next().unwrap()),
            _ => NoneOneOrMany::Many(items),
        }
    }
}

// Create NoneOneOrMany from an iterator
impl<T> FromIterator<T> for NoneOneOrMany<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut iter = iter.into_iter();

        match (iter.next(), iter.next()) {
            (None, _) => Self::None,
            (Some(first), None) => Self::One(first),
            (Some(first), Some(second)) => {
                Self::Many(once(first).chain(once(second)).chain(iter).collect())
            }
        }
    }
}

// Convert NoneOneOrMany into an iterator
impl<T> IntoIterator for NoneOneOrMany<T> {
    type Item = T;
    type IntoIter = Either<std::iter::Empty<T>, Either<std::iter::Once<T>, std::vec::IntoIter<T>>>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            NoneOneOrMany::None => Either::Left(std::iter::empty()),
            NoneOneOrMany::One(item) => Either::Right(Either::Left(std::iter::once(item))),
            NoneOneOrMany::Many(items) => Either::Right(Either::Right(items.into_iter())),
        }
    }
}

// Implement IntoIterator for reference
impl<'a, T> IntoIterator for &'a NoneOneOrMany<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_ref().iter()
    }
}

impl<'a, T> IntoIterator for &'a mut NoneOneOrMany<T> {
    type Item = &'a mut T;
    type IntoIter = Either<
        std::iter::Empty<&'a mut T>,
        Either<std::iter::Once<&'a mut T>, std::slice::IterMut<'a, T>>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            NoneOneOrMany::None => Either::Left(std::iter::empty()),
            NoneOneOrMany::One(item) => Either::Right(Either::Left(std::iter::once(item))),
            NoneOneOrMany::Many(items) => Either::Right(Either::Right(items.iter_mut())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extend_appends_the_right_side_after_the_left() {
        assert_eq!(
            NoneOneOrMany::One(1).extend([2, 3]),
            NoneOneOrMany::Many(vec![1, 2, 3])
        );
        assert_eq!(
            NoneOneOrMany::Many(vec![1, 2]).extend([3]),
            NoneOneOrMany::Many(vec![1, 2, 3])
        );
        assert_eq!(
            NoneOneOrMany::Many(vec![1, 2]).extend([3, 4]),
            NoneOneOrMany::Many(vec![1, 2, 3, 4])
        );
        assert_eq!(
            NoneOneOrMany::One(1).extend([2]),
            NoneOneOrMany::Many(vec![1, 2])
        );
    }

    #[test]
    fn extend_with_nothing_leaves_the_left_side_untouched() {
        assert_eq!(NoneOneOrMany::<u8>::None.extend([]), NoneOneOrMany::None);
        assert_eq!(NoneOneOrMany::One(1).extend([]), NoneOneOrMany::One(1));
        assert_eq!(
            NoneOneOrMany::Many(vec![1, 2]).extend([]),
            NoneOneOrMany::Many(vec![1, 2])
        );
    }

    #[test]
    fn extending_none_lands_in_the_smallest_variant_that_fits() {
        assert_eq!(NoneOneOrMany::None.extend([1]), NoneOneOrMany::One(1));
        assert_eq!(
            NoneOneOrMany::None.extend([1, 2]),
            NoneOneOrMany::Many(vec![1, 2])
        );
    }

    #[test]
    fn extend_leaves_an_empty_many_on_the_left_where_it_found_it() {
        // {"Many":[]} arrives by deserialisation, and extend does not normalise
        // it away. The derived PartialEq, Ord and Hash all read the variant, so
        // Many([1]) and One(1) are different values.
        assert_eq!(
            NoneOneOrMany::Many(Vec::new()).extend([1]),
            NoneOneOrMany::Many(vec![1])
        );
        assert_ne!(NoneOneOrMany::Many(vec![1]), NoneOneOrMany::One(1));
    }

    #[test]
    fn extend_sizes_correctly_when_the_iterator_under_reports() {
        // Every other test feeds an ExactSizeIterator. A filter reports a lower
        // bound of zero, so the Vec cannot be sized from the hint alone.
        let under_reporting = vec![2, 3].into_iter().filter(|_| true);

        assert_eq!(
            NoneOneOrMany::One(1).extend(under_reporting),
            NoneOneOrMany::Many(vec![1, 2, 3])
        );
    }

    #[test]
    fn extend_stepwise_matches_extend_at_once() {
        let stepwise = NoneOneOrMany::None.extend([1]).extend([2]).extend([3]);
        let at_once = NoneOneOrMany::None.extend([1, 2, 3]);

        assert_eq!(stepwise, at_once);
        assert_eq!(stepwise.into_vec(), vec![1, 2, 3]);
    }

    #[test]
    fn extend_buys_one_vec_of_the_size_it_needs() {
        assert!(NoneOneOrMany::One(1).extend([2]).into_vec().capacity() <= 2);
        assert!(NoneOneOrMany::None.extend([1, 2]).into_vec().capacity() <= 2);
        assert!(NoneOneOrMany::One(1).extend([2, 3]).into_vec().capacity() <= 3);
    }

    #[test]
    fn collecting_lands_in_the_smallest_variant_that_fits() {
        assert_eq!(
            NoneOneOrMany::from_iter(std::iter::empty::<u8>()),
            NoneOneOrMany::None
        );
        assert_eq!(NoneOneOrMany::from_iter([1]), NoneOneOrMany::One(1));
        assert_eq!(
            NoneOneOrMany::from_iter([1, 2]),
            NoneOneOrMany::Many(vec![1, 2])
        );
        assert_eq!(
            NoneOneOrMany::from_iter([1, 2, 3]).into_vec(),
            vec![1, 2, 3],
            "collecting preserves order"
        );
    }

    #[test]
    fn an_empty_collection_answers_for_itself() {
        let none = NoneOneOrMany::<u8>::None;

        assert_eq!(none.len(), 0);
        assert!(none.is_empty());
        assert_eq!(none.as_ref(), &[] as &[u8]);
        assert_eq!(none.into_option(), None);
    }
}
