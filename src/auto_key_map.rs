use std::collections::HashMap;
use std::hash::Hash;

use itertools::Itertools;

pub struct AutoKeyMap<Key, T> {
    map: HashMap<Key, T>,
}

pub trait Incrementable {
    fn increment(&self) -> Self;
}

impl<Key, T> AutoKeyMap<Key, T>
where
    Key: Ord + Eq + Hash + Default + Incrementable + Copy,
{
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, value: T) -> Key {
        if self.map.is_empty() {
            self.map.insert(Key::default(), value);
            return Key::default();
        }
        let mut keys = self.map.keys().collect::<Vec<_>>();
        keys.sort();

        let default = Key::default();
        let max = keys.last().map(|key| **key).unwrap_or(default);

        let key = max.increment();
        self.map.insert(key, value);

        key
    }

    pub fn remove(&mut self, key: Key) -> Option<T> {
        self.map.remove(&key)
    }

    pub fn get(&self, key: Key) -> Option<&T> {
        self.map.get(&key)
    }

    pub fn get_mut(&mut self, key: Key) -> Option<&mut T> {
        self.map.get_mut(&key)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        let mut vec = self.map.iter_mut().collect_vec();
        vec.sort_by_key(|(key, _)| **key);
        vec.into_iter().map(|(_, value)| value)
    }

    pub fn entries(&self) -> impl Iterator<Item = (&Key, &T)> {
        let mut vec = self.map.iter().collect_vec();
        vec.sort_by_key(|(key, _)| **key);
        vec.into_iter()
    }

    pub fn keys(&self) -> impl Iterator<Item = &Key> {
        let mut vec = self.map.keys().collect_vec();
        vec.sort();
        vec.into_iter()
    }
}

#[cfg(test)]
mod test_auto_key_map {
    use super::{AutoKeyMap, Incrementable};

    impl Incrementable for usize {
        fn increment(&self) -> Self {
            self + 1
        }
    }

    #[test]
    fn should_auto_increment_keys() {
        let mut map: AutoKeyMap<usize, i32> = AutoKeyMap::new();
        let key1 = map.insert(1);
        let key2 = map.insert(2);
        let key3 = map.insert(3);
        assert_eq!(key1, 0);
        assert_eq!(key2, 1);
        assert_eq!(key3, 2);

        assert_eq!(map.get(key1), Some(&1));
        assert_eq!(map.get(key2), Some(&2));
        assert_eq!(map.get(key3), Some(&3));
    }

    #[test]
    fn values_mut_should_be_ordered_by_key() {
        let mut map: AutoKeyMap<usize, i32> = AutoKeyMap::new();
        map.insert(1);
        map.insert(2);
        map.insert(3);

        let mut values = map.values_mut();
        assert_eq!(values.next(), Some(&mut 1));
        assert_eq!(values.next(), Some(&mut 2));
        assert_eq!(values.next(), Some(&mut 3));
    }
}
