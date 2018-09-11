use std::collections::HashSet;
//use std::mem::swap;
use std::ops::{Not, BitAnd, BitOr};

use document::Primary;

#[derive(Clone)]
pub struct Selection {
    // None is Universe
    pub inner: Option<HashSet<Primary>>,
    // None is Nothing
    pub outer: Option<HashSet<Primary>>,
}

impl Selection {
    pub fn new(ids: HashSet<Primary>) -> Self {
        Selection { inner: Some(ids), outer: None }
    }

    pub fn normalize(&mut self) {
        match (&mut self.inner, &mut self.outer) {
            (Some(inner), Some(outer)) => {
                *inner = inner.difference(outer).cloned().collect();
                *outer = outer.difference(inner).cloned().collect();
            },
            _ => (),
        }
    }

    pub fn normalized(mut self) -> Self {
        self.normalize();
        self
    }

    pub fn has(&self, id: &Primary) -> bool {
        match (&self.inner, &self.outer) {
            (Some(inner), Some(outer)) => inner.contains(id) || !outer.contains(id),
            (Some(inner), None) => inner.contains(id),
            (None, Some(outer)) => !outer.contains(id),
            _ => true,
        }
    }
}

impl Default for Selection {
    fn default() -> Self {
        Selection { inner: None, outer: None }
    }
}

impl Not for Selection {
    type Output = Self;

    fn not(self) -> Self::Output {
        //swap(&mut self.inner, &mut self.outer);
        let Selection { inner, outer } = self;
        Selection { inner: outer, outer: inner }
    }
}

impl BitAnd for Selection {
    type Output = Self;

    fn bitand(self, other: Self) -> Self::Output {
        let Selection { inner: inner_a, outer: outer_a } = self;
        let Selection { inner: inner_b, outer: outer_b } = other;
        
        let inner = match (inner_a, inner_b) {
            (None, None) => None,
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (Some(a), Some(b)) => Some(a.intersection(&b).cloned().collect::<HashSet<_>>()),
        };

        let outer = match (outer_a, outer_b) {
            (None, None) => None,
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (Some(a), Some(b)) => Some(a.union(&b).cloned().collect::<HashSet<_>>()),
        };

        Selection { inner, outer }.normalized()
    }
}

impl BitOr for Selection {
    type Output = Self;

    fn bitor(self, other: Self) -> Self::Output {
        let Selection { inner: inner_a, outer: outer_a } = self;
        let Selection { inner: inner_b, outer: outer_b } = other;
        
        let inner = match (inner_a, inner_b) {
            (None, None) => None,
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (Some(a), Some(b)) => Some(a.union(&b).cloned().collect::<HashSet<_>>()),
        };

        let outer = match (outer_a, outer_b) {
            (None, None) => None,
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (Some(a), Some(b)) => Some(a.intersection(&b).cloned().collect::<HashSet<_>>()),
        };

        Selection { inner, outer }.normalized()
    }
}
