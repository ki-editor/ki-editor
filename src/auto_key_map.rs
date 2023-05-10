use std::collections::HashMap;

pub struct AutoKeyMap<T> {
    map: HashMap<usize, T>,
}

impl<T> AutoKeyMap<T> {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, value: T) -> usize {
        if self.map.is_empty() {
            self.map.insert(0, value);
            return 0;
        }
        let mut keys = self.map.keys().collect::<Vec<_>>();
        keys.sort();

        let max = keys.last().unwrap_or(&&0);

        let key = *max + 1;
        self.map.insert(key, value);

        key
    }

    pub fn remove(&mut self, key: usize) -> Option<T> {
        self.map.remove(&key)
    }

    pub fn get(&self, key: usize) -> Option<&T> {
        self.map.get(&key)
    }

    pub fn get_mut(&mut self, key: usize) -> Option<&mut T> {
        self.map.get_mut(&key)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&usize, &mut T)> {
        self.map.iter_mut()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.map.values_mut()
    }
}

#[cfg(test)]
mod test_auto_key_map {
    #[test]
    fn should_auto_increment_keys() {
        let mut map = super::AutoKeyMap::new();
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
}
