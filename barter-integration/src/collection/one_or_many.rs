use crate::collection::none_one_or_many::NoneOneOrMany;
use itertools::Either;
use serde::{Deserialize, Serialize};
use std::{
    borrow::{Borrow, BorrowMut},
    convert::AsRef,
    iter::once,
};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

// Utility methods
impl<T> OneOrMany<T> {
    pub fn map<U, F>(self, f: F) -> OneOrMany<U>
    where
        F: Fn(T) -> U,
    {
        match self {
            Self::One(x) => OneOrMany::One(f(x)),
            Self::Many(vec) => OneOrMany::Many(vec.into_iter().map(f).collect()),
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
            Self::One(value) => value == item,
            Self::Many(values) => values.contains(item),
        }
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        match self {
            Self::One(_) => 1,
            Self::Many(items) => items.len(),
        }
    }

    pub fn is_one(&self) -> bool {
        matches!(self, Self::One(_))
    }

    pub fn is_many(&self) -> bool {
        matches!(self, Self::Many(_))
    }

    pub fn into_vec(self) -> Vec<T> {
        match self {
            Self::One(item) => vec![item],
            Self::Many(items) => items,
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.as_ref().iter()
    }
}

// Default implementation if T implements Default
impl<T: Default> Default for OneOrMany<T> {
    fn default() -> Self {
        OneOrMany::One(T::default())
    }
}

// AsRef implementation for slice access
impl<T> AsRef<[T]> for OneOrMany<T> {
    fn as_ref(&self) -> &[T] {
        match self {
            Self::One(item) => std::slice::from_ref(item),
            Self::Many(items) => items.as_slice(),
        }
    }
}

// Borrow implementation for slice access
impl<T> Borrow<[T]> for OneOrMany<T> {
    fn borrow(&self) -> &[T] {
        self.as_ref()
    }
}

// BorrowMut implementation for mutable slice access
impl<T> BorrowMut<[T]> for OneOrMany<T> {
    fn borrow_mut(&mut self) -> &mut [T] {
        match self {
            Self::One(item) => std::slice::from_mut(item),
            Self::Many(items) => items.as_mut_slice(),
        }
    }
}

// From implementations for various types
impl<T> From<T> for OneOrMany<T> {
    fn from(item: T) -> Self {
        OneOrMany::One(item)
    }
}

impl<T> From<Vec<T>> for OneOrMany<T> {
    fn from(mut items: Vec<T>) -> Self {
        match items.len() {
            0 => panic!("Cannot create OneOrMany from empty Vec"),
            1 => OneOrMany::One(items.remove(0)),
            _ => OneOrMany::Many(items),
        }
    }
}

impl<T> From<NoneOneOrMany<T>> for Option<OneOrMany<T>> {
    fn from(value: NoneOneOrMany<T>) -> Self {
        match value {
            NoneOneOrMany::None => None,
            NoneOneOrMany::One(value) => Some(OneOrMany::One(value)),
            NoneOneOrMany::Many(values) => Some(OneOrMany::Many(values)),
        }
    }
}

// FromIterator implementation
impl<T> FromIterator<T> for OneOrMany<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut iter = iter.into_iter();

        match (iter.next(), iter.next()) {
            (None, _) => Self::Many(Vec::new()),
            (Some(first), None) => Self::One(first),
            (Some(first), Some(second)) => {
                Self::Many(once(first).chain(once(second)).chain(iter).collect())
            }
        }
    }
}

// IntoIterator implementation
impl<T> IntoIterator for OneOrMany<T> {
    type Item = T;
    type IntoIter = Either<std::iter::Once<T>, std::vec::IntoIter<T>>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            OneOrMany::One(item) => Either::Left(std::iter::once(item)),
            OneOrMany::Many(items) => Either::Right(items.into_iter()),
        }
    }
}

// IntoIterator implementation for references
impl<'a, T> IntoIterator for &'a OneOrMany<T> {
    type Item = &'a T;
    type IntoIter = Either<std::iter::Once<&'a T>, std::slice::Iter<'a, T>>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            OneOrMany::One(item) => Either::Left(std::iter::once(item)),
            OneOrMany::Many(items) => Either::Right(items.iter()),
        }
    }
}

// IntoIterator implementation for mutable references
impl<'a, T> IntoIterator for &'a mut OneOrMany<T> {
    type Item = &'a mut T;
    type IntoIter = Either<std::iter::Once<&'a mut T>, std::slice::IterMut<'a, T>>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            OneOrMany::One(item) => Either::Left(std::iter::once(item)),
            OneOrMany::Many(items) => Either::Right(items.iter_mut()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extend_appends_the_right_side_after_the_left() {
        assert_eq!(
            OneOrMany::One(1).extend([2, 3]),
            OneOrMany::Many(vec![1, 2, 3])
        );
        assert_eq!(
            OneOrMany::Many(vec![1, 2]).extend([3]),
            OneOrMany::Many(vec![1, 2, 3])
        );
        assert_eq!(
            OneOrMany::Many(vec![1, 2]).extend([3, 4]),
            OneOrMany::Many(vec![1, 2, 3, 4])
        );
        assert_eq!(OneOrMany::One(1).extend([2]), OneOrMany::Many(vec![1, 2]));
    }

    #[test]
    fn extend_with_nothing_leaves_the_left_side_untouched() {
        assert_eq!(OneOrMany::One(1).extend([]), OneOrMany::One(1));
        assert_eq!(
            OneOrMany::Many(vec![1, 2]).extend([]),
            OneOrMany::Many(vec![1, 2])
        );
    }

    #[test]
    #[should_panic(expected = "Cannot create OneOrMany from empty Vec")]
    fn from_an_empty_vec_panics_where_from_iter_answers_with_an_empty_many() {
        // The two doors into the type disagree, and only the from_iter side was
        // pinned. Both are now held still while which one is right is undecided.
        let _ = OneOrMany::<u8>::from(Vec::new());
    }

    #[test]
    fn extend_leaves_an_empty_many_on_the_left_where_it_found_it() {
        // {"Many":[]} arrives by deserialisation, and extend does not normalise
        // it away. The derived PartialEq, Ord and Hash all read the variant, so
        // Many([1]) and One(1) are different values.
        assert_eq!(
            OneOrMany::Many(Vec::new()).extend([1]),
            OneOrMany::Many(vec![1])
        );
        assert_ne!(OneOrMany::Many(vec![1]), OneOrMany::One(1));
    }

    #[test]
    fn extend_sizes_correctly_when_the_iterator_under_reports() {
        // Every other test feeds an ExactSizeIterator. A filter reports a lower
        // bound of zero, so the Vec cannot be sized from the hint alone.
        let under_reporting = vec![2, 3].into_iter().filter(|_| true);

        assert_eq!(
            OneOrMany::One(1).extend(under_reporting),
            OneOrMany::Many(vec![1, 2, 3])
        );
    }

    #[test]
    fn extend_stepwise_matches_extend_at_once() {
        let stepwise = OneOrMany::One(1).extend([2]).extend([3]);
        let at_once = OneOrMany::One(1).extend([2, 3]);

        assert_eq!(stepwise, at_once);
        assert_eq!(stepwise.into_vec(), vec![1, 2, 3]);
    }

    #[test]
    fn extend_buys_one_vec_of_the_size_it_needs() {
        assert!(OneOrMany::One(1).extend([2]).into_vec().capacity() <= 2);
        assert!(OneOrMany::One(1).extend([2, 3]).into_vec().capacity() <= 3);
    }

    #[test]
    fn collecting_one_item_carries_no_vec() {
        let one = OneOrMany::from_iter([1]);

        assert_eq!(one, OneOrMany::One(1));
        assert!(one.is_one());
    }

    #[test]
    fn collecting_lands_in_the_variant_the_count_calls_for() {
        assert_eq!(OneOrMany::from_iter([1, 2]), OneOrMany::Many(vec![1, 2]));
        assert_eq!(
            OneOrMany::from_iter([1, 2, 3]).into_vec(),
            vec![1, 2, 3],
            "collecting preserves order"
        );
        // Undecided: the type says non-empty, but From<Vec<T>> panics on empty
        // while this door returns an empty Many. Pinned so it cannot drift
        // silently before that is settled.
        assert_eq!(
            OneOrMany::from_iter(std::iter::empty::<u8>()),
            OneOrMany::Many(vec![])
        );
    }

    #[test]
    fn a_single_item_round_trips_through_the_iterator() {
        let one = OneOrMany::One(1);

        assert_eq!(one.len(), 1);
        assert!(one.is_one());
        assert_eq!(one.as_ref(), &[1]);
        assert_eq!(one.into_iter().collect::<Vec<_>>(), vec![1]);
    }
}
