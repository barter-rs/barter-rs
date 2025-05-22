use crate::collection::none_one_or_many::NoneOneOrMany;
use itertools::Either;
use serde::{Deserialize, Serialize};
use std::{
    borrow::{Borrow, BorrowMut},
    convert::AsRef,
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

    pub fn extend<Iter>(self, other: Iter) -> Self
    where
        Iter: IntoIterator<Item = T>,
    {
        let other = Self::from_iter(other);

        use OneOrMany::*;
        match (self, other) {
            (One(left), One(right)) => Many(vec![left, right]),
            (One(left), Many(mut right)) => {
                right.push(left);
                Many(right)
            }
            (Many(mut left), One(right)) => {
                left.push(right);
                Many(left)
            }
            (Many(mut left), Many(right)) => {
                left.extend(right);
                Many(left)
            }
        }
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
        let mut collection = iter.into_iter().collect::<Vec<_>>();
        match collection.len() {
            1 => Self::One(collection.swap_remove(0)),
            _ => Self::Many(collection),
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
