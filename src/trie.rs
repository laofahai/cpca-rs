//! 前缀树实现，用于高效地名匹配

use std::collections::HashMap;

/// 前缀树节点
#[derive(Debug)]
pub struct TrieNode<T> {
    /// 子节点映射（字符 -> 子节点）
    children: HashMap<char, TrieNode<T>>,
    /// 如果当前节点是一个完整词的结尾，存储关联的数据
    value: Option<T>,
    /// 是否是词的结尾
    is_end: bool,
}

impl<T> Default for TrieNode<T> {
    fn default() -> Self {
        Self {
            children: HashMap::new(),
            value: None,
            is_end: false,
        }
    }
}

/// 前缀树，用于快速匹配地名
#[derive(Debug)]
pub struct Trie<T> {
    root: TrieNode<T>,
}

impl<T> Default for Trie<T> {
    fn default() -> Self {
        Self {
            root: TrieNode::default(),
        }
    }
}

impl<T: Clone> Trie<T> {
    /// 创建空的前缀树
    pub fn new() -> Self {
        Self {
            root: TrieNode::default(),
        }
    }

    /// 插入一个词及其关联数据
    pub fn insert(&mut self, word: &str, value: T) {
        let mut node = &mut self.root;
        for ch in word.chars() {
            node = node.children.entry(ch).or_default();
        }
        node.is_end = true;
        node.value = Some(value);
    }

    /// 查找精确匹配
    #[allow(dead_code)]
    pub fn get(&self, word: &str) -> Option<&T> {
        let mut node = &self.root;
        for ch in word.chars() {
            match node.children.get(&ch) {
                Some(n) => node = n,
                None => return None,
            }
        }
        if node.is_end {
            node.value.as_ref()
        } else {
            None
        }
    }

    /// 检查是否存在
    #[allow(dead_code)]
    pub fn contains(&self, word: &str) -> bool {
        self.get(word).is_some()
    }

    /// 从文本开头查找最长匹配
    ///
    /// 返回 (匹配的词, 关联数据, 匹配长度)
    pub fn find_longest_prefix<'a>(&self, text: &'a str) -> Option<(&'a str, &T, usize)> {
        let mut node = &self.root;
        let mut last_match: Option<(&'a str, &T, usize)> = None;
        let mut current_len = 0;

        for ch in text.chars() {
            match node.children.get(&ch) {
                Some(n) => {
                    node = n;
                    current_len += ch.len_utf8();
                    if node.is_end {
                        if let Some(ref value) = node.value {
                            last_match = Some((&text[..current_len], value, current_len));
                        }
                    }
                }
                None => break,
            }
        }

        last_match
    }

    /// 查找文本中所有匹配的词
    ///
    /// 返回 Vec<(起始位置, 匹配词, 关联数据)>
    #[allow(dead_code)]
    pub fn find_all<'a>(&self, text: &'a str) -> Vec<(usize, &'a str, &T)> {
        let mut results = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        let mut byte_pos = 0;

        for (i, _) in chars.iter().enumerate() {
            // 从当前位置开始尝试匹配
            let remaining = &text[byte_pos..];
            if let Some((matched, value, _)) = self.find_longest_prefix(remaining) {
                results.push((byte_pos, matched, value));
            }
            byte_pos += chars[i].len_utf8();
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trie_insert_and_get() {
        let mut trie = Trie::new();
        trie.insert("广东省", 1);
        trie.insert("广东", 2);
        trie.insert("广州市", 3);

        assert_eq!(trie.get("广东省"), Some(&1));
        assert_eq!(trie.get("广东"), Some(&2));
        assert_eq!(trie.get("广州市"), Some(&3));
        assert_eq!(trie.get("广"), None);
        assert_eq!(trie.get("北京"), None);
    }

    #[test]
    fn test_find_longest_prefix() {
        let mut trie = Trie::new();
        trie.insert("广东", 1);
        trie.insert("广东省", 2);

        let text = "广东省深圳市";
        let result = trie.find_longest_prefix(text);
        assert!(result.is_some());
        let (matched, value, len) = result.unwrap();
        assert_eq!(matched, "广东省");
        assert_eq!(*value, 2);
        assert_eq!(len, "广东省".len());
    }

    #[test]
    fn test_find_all() {
        let mut trie = Trie::new();
        trie.insert("广东省", 1);
        trie.insert("深圳市", 2);
        trie.insert("南山区", 3);

        let text = "广东省深圳市南山区科技园";
        let results = trie.find_all(text);

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].1, "广东省");
        assert_eq!(results[1].1, "深圳市");
        assert_eq!(results[2].1, "南山区");
    }
}
