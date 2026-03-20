/// 从文本中提取所有符合条件的词
///
/// # Arguments
/// * `content` - 要提取词的文本内容
/// * `min_length` - 最小词长度，短于此长度的词将被忽略
///
/// # Returns
/// 提取出的词列表（已去重）
pub fn extract_words(content: &str, min_length: usize) -> Vec<String> {
    let mut words = Vec::new();
    let mut current_word = String::new();

    for ch in content.chars() {
        if is_word_char(ch) {
            current_word.push(ch);
        } else {
            if current_word.len() >= min_length {
                words.push(current_word.clone());
            }
            current_word.clear();
        }
    }

    // 处理最后一个词
    if current_word.len() >= min_length {
        words.push(current_word);
    }

    words
}

/// 判断字符是否为词字符
///
/// 词字符包括：字母、数字、下划线、连字符
fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == '-'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_words_basic() {
        let content = "hello world foo bar";
        let words = extract_words(content, 3);
        assert_eq!(words, vec!["hello", "world", "foo", "bar"]);
    }

    #[test]
    fn test_extract_words_with_underscores() {
        let content = "hello_world foo_bar";
        let words = extract_words(content, 3);
        assert_eq!(words, vec!["hello_world", "foo_bar"]);
    }

    #[test]
    fn test_extract_words_with_hyphens() {
        let content = "hello-world foo-bar";
        let words = extract_words(content, 3);
        assert_eq!(words, vec!["hello-world", "foo-bar"]);
    }

    #[test]
    fn test_extract_words_min_length() {
        let content = "hi hello world";
        let words = extract_words(content, 3);
        assert_eq!(words, vec!["hello", "world"]);
    }

    #[test]
    fn test_extract_words_empty() {
        let content = "";
        let words = extract_words(content, 3);
        assert!(words.is_empty());
    }

    #[test]
    fn test_extract_words_only_short() {
        let content = "a b c d";
        let words = extract_words(content, 3);
        assert!(words.is_empty());
    }

    #[test]
    fn test_extract_words_with_numbers() {
        let content = "test123 456test mix3d";
        let words = extract_words(content, 3);
        assert_eq!(words, vec!["test123", "456test", "mix3d"]);
    }

    #[test]
    fn test_extract_words_with_punctuation() {
        let content = "hello, world! foo.bar; baz(qux)";
        let words = extract_words(content, 3);
        assert_eq!(words, vec!["hello", "world", "foo", "bar", "baz", "qux"]);
    }

    #[test]
    fn test_extract_words_multiline() {
        let content = "hello\nworld\nfoo_bar";
        let words = extract_words(content, 3);
        assert_eq!(words, vec!["hello", "world", "foo_bar"]);
    }

    #[test]
    fn test_extract_words_chinese() {
        // 中文字符也是 alphanumeric
        let content = "你好世界 hello";
        let words = extract_words(content, 3);
        assert_eq!(words, vec!["你好世界", "hello"]);
    }
}
