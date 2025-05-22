use crate::collection::one_or_many::OneOrMany;
use itertools::Either;
use serde::{Deserialize, Serialize};
use std::{
    borrow::{Borrow, BorrowMut},
    iter::{FromIterator, IntoIterator},
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

    pub fn extend<Iter>(self, other: Iter) -> Self
    where
        Iter: IntoIterator<Item = T>,
    {
        let other = Self::from_iter(other);

        use NoneOneOrMany::*;
        match (self, other) {
            (None, right) => right,
            (left, None) => left,
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
        let mut collection = iter.into_iter().collect::<Vec<_>>();
        match collection.len() {
            0 => Self::None,
            1 => Self::One(collection.swap_remove(0)),
            _ => Self::Many(collection),
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
