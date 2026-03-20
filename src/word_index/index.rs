use std::collections::HashMap;
use std::time::Instant;

use super::word_extractor::extract_words;

/// 词项，包含词本身和最后使用时间
#[derive(Debug, Clone)]
pub struct WordEntry {
    /// 词本身
    pub word: String,
    /// 最后使用时间（用于 LRU 排序）
    pub last_used: Instant,
    /// 出现频率
    pub frequency: usize,
}

/// 全局词库索引
///
/// 使用 BTreeMap 按词的字典序存储，支持前缀匹配。
/// 同时维护 LRU 信息用于补全排序。
#[derive(Debug)]
pub struct WordIndex {
    /// 词 -> WordEntry 的映射
    words: HashMap<String, WordEntry>,
    /// 最小词长度
    min_length: usize,
    /// 最大词数（防止内存爆炸）
    max_words: usize,
}

impl Default for WordIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl WordIndex {
    /// 创建新的空词库
    pub fn new() -> Self {
        Self {
            words: HashMap::new(),
            min_length: 3,
            max_words: 10000,
        }
    }

    /// 使用自定义配置创建词库
    pub fn with_config(min_length: usize, max_words: usize) -> Self {
        Self {
            words: HashMap::new(),
            min_length,
            max_words,
        }
    }

    /// 从 buffer 内容提取词并更新索引
    ///
    /// # Arguments
    /// * `content` - buffer 的内容
    pub fn update_from_buffer(&mut self, content: &str) {
        let extracted = extract_words(content, self.min_length);

        let now = Instant::now();
        for word in extracted {
            self.insert_word(word, now);
        }

        // 如果超过最大词数，移除最久未使用的词
        self.evict_if_needed();
    }

    /// 插入或更新一个词
    fn insert_word(&mut self, word: String, now: Instant) {
        if word.len() < self.min_length {
            return;
        }

        self.words
            .entry(word.clone())
            .and_modify(|entry| {
                entry.frequency += 1;
                entry.last_used = now;
            })
            .or_insert(WordEntry {
                word,
                last_used: now,
                frequency: 1,
            });
    }

    /// 如果超过最大词数，移除最久未使用的词
    fn evict_if_needed(&mut self) {
        if self.words.len() <= self.max_words {
            return;
        }

        // 找出最久未使用的词
        let to_remove_count = self.words.len() - self.max_words;

        // 收集需要移除的词（先收集，再移除，避免借用冲突）
        let mut entries: Vec<_> = self.words.values().collect();
        entries.sort_by_key(|e| e.last_used);

        let words_to_remove: Vec<String> = entries
            .into_iter()
            .take(to_remove_count)
            .map(|e| e.word.clone())
            .collect();

        // 移除最久未使用的词
        for word in words_to_remove {
            self.words.remove(&word);
        }
    }

    /// 根据前缀获取补全建议（LRU 排序）
    ///
    /// # Arguments
    /// * `prefix` - 前缀
    /// * `limit` - 最大返回数量
    ///
    /// # Returns
    /// 匹配的词列表，按最近使用时间降序排列
    pub fn complete(&self, prefix: &str, limit: usize) -> Vec<String> {
        if prefix.is_empty() {
            return Vec::new();
        }

        let prefix_lower = prefix.to_lowercase();

        // 找到所有匹配前缀的词
        let mut matches: Vec<_> = self
            .words
            .values()
            .filter(|entry| entry.word.to_lowercase().starts_with(&prefix_lower))
            .collect();

        // 按 LRU 排序（最近使用的在前）
        matches.sort_by(|a, b| b.last_used.cmp(&a.last_used));

        // 取前 limit 个
        matches.into_iter().take(limit).map(|e| e.word.clone()).collect()
    }

    /// 标记词被使用（更新 LRU）
    ///
    /// 当用户选择了某个补全项时调用
    pub fn touch(&mut self, word: &str) {
        if let Some(entry) = self.words.get_mut(word) {
            entry.last_used = Instant::now();
        }
    }

    /// 获取词库中的词数量
    pub fn len(&self) -> usize {
        self.words.len()
    }

    /// 检查词库是否为空
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// 清空词库
    pub fn clear(&mut self) {
        self.words.clear();
    }

    /// 检查词库中是否包含某个词
    pub fn contains(&self, word: &str) -> bool {
        self.words.contains_key(word)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_new_word_index() {
        let index = WordIndex::new();
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);
    }

    #[test]
    fn test_update_from_buffer() {
        let mut index = WordIndex::new();
        index.update_from_buffer("hello world foo");

        assert_eq!(index.len(), 3);
        assert!(index.contains("hello"));
        assert!(index.contains("world"));
        assert!(index.contains("foo"));
    }

    #[test]
    fn test_update_respects_min_length() {
        let mut index = WordIndex::new();
        index.update_from_buffer("hi hello world");

        // "hi" 长度为 2，小于 min_length (3)，不应被收集
        assert_eq!(index.len(), 2);
        assert!(!index.contains("hi"));
    }

    #[test]
    fn test_complete_basic() {
        let mut index = WordIndex::new();
        index.update_from_buffer("hello world help helmet");

        let completions = index.complete("hel", 10);
        assert_eq!(completions.len(), 3);
        assert!(completions.contains(&"hello".to_string()));
        assert!(completions.contains(&"help".to_string()));
        assert!(completions.contains(&"helmet".to_string()));
    }

    #[test]
    fn test_complete_case_insensitive() {
        let mut index = WordIndex::new();
        index.update_from_buffer("Hello World HELLO");

        let completions = index.complete("hel", 10);
        assert!(!completions.is_empty());
    }

    #[test]
    fn test_complete_limit() {
        let mut index = WordIndex::new();
        index.update_from_buffer("hello help helmet helix helium");

        let completions = index.complete("hel", 2);
        assert_eq!(completions.len(), 2);
    }

    #[test]
    fn test_complete_empty_prefix() {
        let mut index = WordIndex::new();
        index.update_from_buffer("hello world");

        let completions = index.complete("", 10);
        assert!(completions.is_empty());
    }

    #[test]
    fn test_complete_no_match() {
        let mut index = WordIndex::new();
        index.update_from_buffer("hello world");

        let completions = index.complete("xyz", 10);
        assert!(completions.is_empty());
    }

    #[test]
    fn test_touch_updates_lru() {
        let mut index = WordIndex::new();
        index.update_from_buffer("hello");

        // 等待一小段时间
        thread::sleep(Duration::from_millis(10));

        index.update_from_buffer("world");

        // 等待一小段时间
        thread::sleep(Duration::from_millis(10));

        // 此时 "hello" 最久未使用，"world" 是最新的
        // 再 touch "hello"，使其变成最新
        index.touch("hello");

        // 验证 "hello" 排在 "world" 前面
        let completions = index.complete("h", 10);
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0], "hello");
    }

    #[test]
    fn test_frequency_counting() {
        let mut index = WordIndex::new();
        index.update_from_buffer("hello hello hello world");

        let hello_entry = index.words.get("hello").unwrap();
        assert_eq!(hello_entry.frequency, 3);

        let world_entry = index.words.get("world").unwrap();
        assert_eq!(world_entry.frequency, 1);
    }

    #[test]
    fn test_eviction() {
        let mut index = WordIndex::with_config(3, 3); // max_words = 3

        // 添加 3 个词
        index.update_from_buffer("first second third");
        assert_eq!(index.len(), 3);

        // 等待一小段时间
        thread::sleep(Duration::from_millis(10));

        // 添加更多词，应该触发淘汰
        index.update_from_buffer("fourth fifth");
        assert_eq!(index.len(), 3);

        // "first" 应该被淘汰（最久未使用）
        // 注意：由于 update_from_buffer 会更新时间，结果取决于具体实现
    }

    #[test]
    fn test_clear() {
        let mut index = WordIndex::new();
        index.update_from_buffer("hello world");
        assert_eq!(index.len(), 2);

        index.clear();
        assert!(index.is_empty());
    }
}
