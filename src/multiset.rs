use std::collections::BTreeMap;

pub struct MultiSet<T> (BTreeMap<T, usize>);

impl<T: Ord> MultiSet<T> {
    pub fn new() -> Self {
        MultiSet (BTreeMap::new())
    }

    pub fn insert(&mut self, item: T) -> usize {
        let count = match self.0.get(&item) {
            Some(&count) => count,
            None => 0,
        };
        self.0.insert(item, count + 1);
        count
    }
}