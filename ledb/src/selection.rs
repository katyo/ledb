use std::collections::HashSet;
use std::ops::{Not, BitAnd, BitOr};

use super::{Primary, Result};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    pub(crate) ids: HashSet<Primary>,
    pub(crate) inv: bool
}

impl Selection {
    pub fn new(ids: HashSet<Primary>, inv: bool) -> Self {
        Selection { ids, inv }
    }

    pub fn has(&self, id: &Primary) -> bool {
        match self.inv {
            false => self.ids.contains(id),
            true => !self.ids.contains(id),
        }
    }

    pub fn filter<I: Iterator<Item = Result<Primary>>>(self, iter: I) -> impl Iterator<Item = Result<Primary>> {
        iter.filter(move |res| if let Ok(id) = res {
            self.has(&id)
        } else {
            true
        })
    }
}

impl<T: AsRef<[Primary]>> From<T> for Selection {
    fn from(v: T) -> Self {
        Selection::new(v.as_ref().iter().cloned().collect(), false)
    }
}

impl Not for Selection {
    type Output = Self;

    fn not(self) -> Self::Output {
        let Selection { ids, inv } = self;
        Selection { ids, inv: !inv }
    }
}

impl BitAnd for Selection {
    type Output = Self;

    fn bitand(self, other: Self) -> Self::Output {
        let (ids, inv) = match (self.inv, self.ids.len(), other.inv, other.ids.len()) {
            // a & b
            (false, _, false, _) => (self.ids.intersection(&other.ids).cloned().collect(), false),
            // a & universe == a
            (false, _, true, 0) => (self.ids, false),
            // a & !b
            (false, n, true, m) if n < m => (self.ids.difference(&other.ids).cloned().collect(), false),
            // a & !b == !(b | !a)
            (false, _, true, _) => (other.ids.difference(&self.ids).cloned().collect(), true),
            // universe & b == b
            (true, 0, false, _) => (other.ids, false),
            // !a & b == b & !a
            (true, n, false, m) if m < n => (other.ids.difference(&self.ids).cloned().collect(), false),
            // !a & b == !(a | !b)
            (true, _, false, _) => (self.ids.difference(&other.ids).cloned().collect(), true),
            // !a | !b
            (true, _, true, _) => (self.ids.union(&other.ids).cloned().collect(), true),
        };

        Selection::new(ids, inv)
    }
}

impl BitOr for Selection {
    type Output = Self;

    fn bitor(self, other: Self) -> Self::Output {
        // a | b <=> !(!a & !b)
        !(!self & !other)
    }
}

#[cfg(test)]
mod test {
    use super::Selection;

    #[test]
    fn not_inv_and_empty() {
        assert_eq!(Selection::from(&[1, 2, 3, 7, 9]) &
                   Selection::default(),
                   Selection::default());
    }

    #[test]
    fn not_inv_and_universe() {
        assert_eq!(Selection::from(&[1, 2, 3, 7, 9]) &
                   !Selection::default(),
                   Selection::from(&[1, 2, 3, 7, 9]));
    }
    
    #[test]
    fn not_inv_and_not_inv() {
        assert_eq!(Selection::from(&[1, 2, 3, 7, 9]) &
                   Selection::from(&[2, 7, 5, 0, 4, 1]),
                   Selection::from(&[1, 2, 7]));
    }

    #[test]
    fn not_inv_and_inv() {
        assert_eq!(Selection::from(&[1, 2, 3, 7, 9]) &
                   !Selection::from(&[2, 7, 5, 0, 4, 1]),
                   Selection::from(&[3, 9]));
    }
    
    #[test]
    fn inv_and_not_inv() {
        assert_eq!(Selection::from(&[2, 7, 5, 0, 4, 1]) &
                   !Selection::from(&[1, 2, 3, 7, 9]),
                   !Selection::from(&[9, 3]));
    }

    #[test]
    fn inv_and_inv() {
        assert_eq!(!Selection::from(&[1, 2, 3, 7, 9]) &
                   !Selection::from(&[2, 7, 5, 0, 4, 1]),
                   !Selection::from(&[0, 1, 2, 3, 4, 5, 7, 9]));
    }

    #[test]
    fn not_inv_or_empty() {
        assert_eq!(Selection::from(&[1, 2, 3, 7, 9]) |
                   Selection::default(),
                   Selection::from(&[1, 2, 3, 7, 9]));
    }

    #[test]
    fn not_inv_or_universe() {
        assert_eq!(Selection::from(&[1, 2, 3, 7, 9]) |
                   !Selection::default(),
                   !Selection::default());
    }

    #[test]
    fn not_inv_or_not_inv() {
        assert_eq!(Selection::from(&[1, 2, 3, 7, 9]) |
                   Selection::from(&[2, 7, 5, 0, 4, 1]),
                   Selection::from(&[0, 1, 2, 3, 4, 5, 7, 9]));
    }

    #[test]
    fn not_inv_or_inv() {
        assert_eq!(Selection::from(&[1, 2, 3, 7, 9]) |
                   !Selection::from(&[2, 7, 5, 0, 4, 1]),
                   Selection::from(&[3, 9]));
    }

    #[test]
    fn inv_or_not_inv() {
        assert_eq!(!Selection::from(&[2, 7, 5, 0, 4, 1]) |
                   Selection::from(&[1, 2, 3, 7, 9]),
                   Selection::from(&[3, 9]));
    }

    #[test]
    fn inv_or_inv() {
        assert_eq!(!Selection::from(&[1, 2, 3, 7, 9]) |
                   !Selection::from(&[2, 7, 5, 0, 4, 1]),
                   !Selection::from(&[1, 2, 7]));
    }
}
