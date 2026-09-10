//! 地址解析器核心实现

use crate::data::{
    address_aliases, legacy_metadata, load_regions, province_aliases, NameMetadata, RegionIndex,
};
use crate::region::{AddressValidation, CheckedAddress, ParsedAddress, RecognitionStatus, Region};
use crate::trie::Trie;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};

/// 全局解析器实例
static GLOBAL_PARSER: Lazy<AddressParser> = Lazy::new(AddressParser::new);

/// 地址解析器
///
/// 使用前缀树和多级索引实现高效的地址解析。
pub struct AddressParser {
    /// 省份前缀树（含简称）
    province_trie: Trie<String>,
    /// 城市前缀树（含简称）
    city_trie: Trie<String>,
    /// 区县前缀树（含简称）
    district_trie: Trie<String>,
    /// 区域索引
    index: RegionIndex,
    district_aliases: HashMap<String, Vec<String>>,
    legacy: HashMap<Region, NameMetadata>,
    reviewed_district_aliases: HashSet<String>,
    /// 省份简称映射
    province_aliases: HashMap<&'static str, &'static str>,
}

fn is_separator(c: char) -> bool {
    c.is_whitespace() || matches!(c, '-' | ',' | '，' | '、' | '/')
}

fn is_road_suffix(tail: &str) -> bool {
    let tail = tail
        .strip_prefix(['东', '西', '南', '北', '中'])
        .unwrap_or(tail);
    ["路", "街", "巷", "大道", "大街", "胡同", "风情街"]
        .iter()
        .any(|suffix| tail.starts_with(suffix))
}

impl AddressParser {
    /// 创建新的解析器实例
    pub fn new() -> Self {
        let regions = load_regions();
        let index = RegionIndex::build(&regions);
        let aliases = province_aliases();

        // 构建省份前缀树
        let mut province_trie = Trie::new();
        for province in &index.provinces {
            province_trie.insert(province, province.clone());
            // 添加简称
            for (short, full) in &aliases {
                if *full == province {
                    province_trie.insert(short, province.clone());
                }
            }
        }

        // 构建城市前缀树
        let mut city_trie = Trie::new();
        for city in &index.cities {
            city_trie.insert(city, city.clone());
            // 添加简称（去掉"市"）
            if city.ends_with("市") {
                let short = city.trim_end_matches("市");
                city_trie.insert(short, city.clone());
            }
        }

        // 同一简称保留全部全名候选，避免 HashSet 遍历顺序决定结果。
        let mut district_aliases: HashMap<String, Vec<String>> = HashMap::new();
        for district in &index.districts {
            district_aliases
                .entry(district.clone())
                .or_default()
                .push(district.clone());
            for suffix in ["区", "县", "市", "旗", "镇", "街道"] {
                if let Some(short) = district.strip_suffix(suffix) {
                    if short.chars().count() >= 2 {
                        district_aliases
                            .entry(short.to_string())
                            .or_default()
                            .push(district.clone());
                    }
                }
            }
        }
        let mut reviewed_district_aliases = HashSet::new();
        for (alias, level, full) in address_aliases() {
            if level == "city" {
                city_trie.insert(alias, full.to_string());
            } else {
                reviewed_district_aliases.insert(alias.to_string());
                district_aliases
                    .entry(alias.to_string())
                    .or_default()
                    .push(full.to_string());
            }
        }
        let mut district_trie = Trie::new();
        for (alias, names) in &mut district_aliases {
            names.sort();
            names.dedup();
            district_trie.insert(alias, alias.clone());
        }

        Self {
            province_trie,
            city_trie,
            district_trie,
            index,
            district_aliases,
            legacy: legacy_metadata(),
            reviewed_district_aliases,
            province_aliases: aliases,
        }
    }

    /// 解析地址并返回现行、历史、地点、歧义或冲突状态。
    ///
    /// 校验基于包内数据快照，无联网请求；不会自动替换历史名称。
    pub fn parse_with_status(&self, input: &str) -> CheckedAddress {
        let address = self.parse(input);
        let validation = self.validate(&address);
        CheckedAddress {
            address,
            validation,
        }
    }

    /// 对照包内数据校验已解析的省市区组合，不校验门牌或建筑物真实性。
    pub fn validate(&self, address: &ParsedAddress) -> AddressValidation {
        let mut result = AddressValidation {
            status: RecognitionStatus::Incomplete,
            note: "未取得完整省市区归属；不能据此判断名称是否现行。".into(),
            current_name: None,
            candidates: Vec::new(),
        };
        if let Some(district) = &address.district {
            if let Some(parents) = self
                .index
                .district_to_city
                .get(district)
                .filter(|_| self.index.districts.contains(district))
            {
                result.candidates = parents
                    .iter()
                    .filter(|(p, c)| {
                        address.province.as_ref().is_none_or(|known| known == p)
                            && address.city.as_ref().is_none_or(|known| known == c)
                    })
                    .map(|(p, c)| Region::new(p.clone(), c.clone(), Some(district.clone())))
                    .collect();
                result.candidates.sort_by(|a, b| {
                    (&a.province, &a.city, &a.district).cmp(&(&b.province, &b.city, &b.district))
                });
                result.candidates.dedup();
            }
            if result.candidates.is_empty() {
                result.status = RecognitionStatus::NeedsReview;
                result.note = "省市区组合不在数据表中或层级互相矛盾，请核对原地址。".into();
            } else if result.candidates.len() > 1 {
                result.status = RecognitionStatus::Ambiguous;
                result.note = "存在多个归属候选，请补充省份或城市。".into();
            } else if address.is_complete() {
                if let Some(meta) = self.legacy.get(&result.candidates[0]) {
                    result.status = meta.status;
                    result.note = meta.note.clone();
                    result.current_name = meta.current_name.clone();
                } else {
                    result.status = RecognitionStatus::Current;
                    result.note.clear();
                }
            }
            if result.status != RecognitionStatus::Ambiguous {
                result.candidates.clear();
            }
        } else if let Some(city) = &address.city {
            if self
                .index
                .city_to_province
                .get(city)
                .is_none_or(|p| address.province.as_ref().is_some_and(|known| known != p))
            {
                result.status = RecognitionStatus::NeedsReview;
                result.note = "省市组合不在数据表中或层级互相矛盾，请核对原地址。".into();
            }
        } else if address
            .province
            .as_ref()
            .is_some_and(|p| !self.index.provinces.contains(p))
        {
            result.status = RecognitionStatus::NeedsReview;
            result.note = "省份不在数据表中，请核对原地址。".into();
        }
        result
    }

    /// 获取全局解析器实例
    pub fn global() -> &'static AddressParser {
        &GLOBAL_PARSER
    }

    /// 解析地址字符串
    ///
    /// # 参数
    /// * `address` - 待解析的地址字符串
    ///
    /// # 返回
    /// 解析结果，包含省、市、区和详细地址
    ///
    /// # 示例
    /// ```rust
    /// use cpca::AddressParser;
    ///
    /// let parser = AddressParser::new();
    /// let result = parser.parse("广东省深圳市南山区科技园");
    /// assert_eq!(result.province, Some("广东省".to_string()));
    /// ```
    pub fn parse(&self, address: &str) -> ParsedAddress {
        let address = address.trim();
        if address.is_empty() {
            return ParsedAddress::empty();
        }

        let mut result = ParsedAddress::default();
        let mut remaining = address.to_string();

        // 第一步：尝试匹配省份
        if let Some((_matched, normalized, len)) = self
            .province_trie
            .find_longest_prefix(&remaining)
            .filter(|(matched, full, len)| {
                if *matched == full.as_str() {
                    return true;
                }
                let tail = &remaining[*len..];
                let full_child =
                    self.district_trie
                        .find_longest_prefix(tail)
                        .is_some_and(|(name, _, _)| {
                            self.index.districts.contains(name)
                                && self.index.district_to_city[name]
                                    .iter()
                                    .any(|(p, _)| p == *full)
                        })
                        || self.city_trie.find_longest_prefix(tail).is_some_and(
                            |(name, city, _)| {
                                name == city && self.index.city_to_province.get(city) == Some(full)
                            },
                        );
                (!is_road_suffix(tail) || full_child)
                    && !self.city_trie.find_longest_prefix(&remaining).is_some_and(
                        |(city_matched, city_full, city_len)| {
                            city_matched == city_full && city_len > *len
                        },
                    )
            })
        {
            result.province = Some(normalized.clone());
            remaining = remaining[len..].to_string();

            // 直辖市特殊处理：省=市，直接跳到区县匹配
            if self.index.is_municipality(normalized) {
                result.city = Some(normalized.clone());
                if let Some(tail) = remaining.strip_prefix(normalized.as_str()) {
                    remaining = tail.to_string();
                }
            }
        }

        // 第二步：尝试匹配城市（但要先检查是否应该优先匹配区县）
        // 按原文实际匹配长度和全称优先级决策，不让城市简称抢占完整区县名。
        let city_match =
            self.city_trie
                .find_longest_prefix(&remaining)
                .filter(|(matched, full, len)| {
                    result.city.is_none()
                        && (*matched == full.as_str()
                            || !is_road_suffix(&remaining[*len..])
                            || self
                                .index
                                .city_districts
                                .get(full.as_str())
                                .is_some_and(|ds| {
                                    ds.iter().any(|d| remaining[*len..].starts_with(d))
                                }))
                });
        let district_match =
            self.match_district(&remaining, &result)
                .filter(|(matched, full, len)| {
                    self.allow_district(matched, full, &remaining[*len..], &result)
                });

        // 判断是否应该优先使用区县匹配
        let prefer_district = match (&city_match, &district_match) {
            (Some((_, _, city_len)), Some((dist_matched, dist_normalized, dist_len))) => {
                // 如果区县匹配更长，或者区县是完整形式（带后缀），优先使用区县
                *dist_len > *city_len
                    || (*dist_len == *city_len && *dist_matched == dist_normalized.as_str())
                    || self.following_parent_matches(dist_normalized, &remaining[*dist_len..])
            }
            (Some(_), None) => false,
            (None, Some(_)) => true,
            (None, None) => false,
        };

        if prefer_district {
            // 优先处理区县匹配
            if let Some((_matched, dist_normalized, dist_len)) = district_match {
                result.district = Some(dist_normalized.clone());

                remaining = remaining[dist_len..].to_string();
            }
        } else {
            // 正常流程：先匹配城市
            if let Some((matched, normalized, len)) = city_match {
                // 如果已有省份，验证城市是否属于该省
                let valid_city = if let Some(ref province) = result.province {
                    self.index
                        .city_to_province
                        .get(normalized)
                        .map(|p| p == province)
                        .unwrap_or(false)
                } else {
                    true
                };

                if valid_city || matched == normalized {
                    result.city = Some(normalized.clone());

                    remaining = remaining[len..].to_string();
                }
            }
        }

        // 市在省前时先读省份，再继续读取区县。
        if let Some(city) = result.city.as_ref().filter(|_| result.province.is_none()) {
            let tail = remaining.trim_start_matches(is_separator);
            if let Some((matched, province, len)) = self.province_trie.find_longest_prefix(tail) {
                let correct_parent = self.index.city_to_province.get(city) == Some(province);
                if matched == province || correct_parent && self.parent_alias_boundary(&tail[len..])
                {
                    result.province = Some(province.clone());
                    remaining = tail[len..].to_string();
                }
            }
        }

        // 第三步：尝试匹配区县（如果还没匹配到）
        if result.district.is_none() {
            if let Some((matched, normalized, len)) = self
                .match_district(&remaining, &result)
                .filter(|(m, f, n)| self.allow_district(m, f, &remaining[*n..], &result))
            {
                // 验证区县是否合法
                let valid = if let Some(ref city) = result.city {
                    matched == normalized || self.index.validate_district(city, normalized)
                } else {
                    true // 没有城市信息时，先接受
                };

                if valid {
                    result.district = Some(normalized.clone());

                    remaining = remaining[len..].to_string();
                }
            }
        }

        // 区在前时继续读取明确父级，市/省顺序均可；简称必须有边界。
        if result.district.is_some() {
            for _ in 0..2 {
                let tail = remaining.trim_start_matches(is_separator);
                if result.city.is_none() {
                    if let Some((matched, city, len)) = self.city_trie.find_longest_prefix(tail) {
                        if matched == city || self.parent_alias_boundary(&tail[len..]) {
                            result.city = Some(city.clone());
                            remaining = tail[len..].to_string();
                            continue;
                        }
                    }
                }
                if result.province.is_none() {
                    if let Some((matched, province, len)) =
                        self.province_trie.find_longest_prefix(tail)
                    {
                        if matched == province || self.parent_alias_boundary(&tail[len..]) {
                            result.province = Some(province.clone());
                            remaining = tail[len..].to_string();
                            continue;
                        }
                    }
                }
                break;
            }
        }
        // 只在符合全部明确父级且归属唯一时补全，不覆盖原文。
        if let Some(ref district) = result.district {
            if let Some(parents) = self.index.district_to_city.get(district) {
                let candidates: std::collections::HashSet<_> = parents
                    .iter()
                    .filter(|(p, c)| {
                        result.province.as_ref().is_none_or(|known| known == p)
                            && result.city.as_ref().is_none_or(|known| known == c)
                    })
                    .collect();
                if candidates.len() == 1 {
                    let (province, city) = candidates.into_iter().next().unwrap();
                    result.province.get_or_insert_with(|| province.clone());
                    result.city.get_or_insert_with(|| city.clone());
                }
            }
        }
        if result.province.is_none() {
            if let Some(ref city) = result.city {
                result.province = self.index.city_to_province.get(city).cloned();
            }
        }

        // 处理直辖市的特殊情况：省=市
        if let Some(ref province) = result.province {
            if self.index.is_municipality(province) && result.city.is_none() {
                result.city = Some(province.clone());
            }
        }

        // 剩余部分作为详细地址
        result.detail = remaining.trim().to_string();

        result
    }

    // 逆序简称必须有边界，不能吞掉“吉林大学”“山东大厦”中的地名。
    fn parent_alias_boundary(&self, tail: &str) -> bool {
        if is_road_suffix(tail) {
            return false;
        }
        tail.is_empty()
            || tail.starts_with(is_separator)
            || self
                .district_trie
                .find_longest_prefix(tail)
                .is_some_and(|(m, _, _)| self.index.districts.contains(m))
            || self
                .province_trie
                .find_longest_prefix(tail)
                .is_some_and(|(m, full, _)| m == full)
            || self
                .city_trie
                .find_longest_prefix(tail)
                .is_some_and(|(m, full, _)| m == full)
    }

    fn following_parent_matches(&self, district: &str, tail: &str) -> bool {
        let tail = tail.trim_start_matches(is_separator);
        let parents = &self.index.district_to_city[district];
        self.city_trie
            .find_longest_prefix(tail)
            .is_some_and(|(matched, city, len)| {
                (matched == city || self.parent_alias_boundary(&tail[len..]))
                    && parents.iter().any(|(_, c)| c == city)
            })
            || self.province_trie.find_longest_prefix(tail).is_some_and(
                |(matched, province, len)| {
                    (matched == province || self.parent_alias_boundary(&tail[len..]))
                        && parents.iter().any(|(p, _)| p == province)
                },
            )
    }

    fn match_district<'a>(
        &'a self,
        text: &'a str,
        result: &ParsedAddress,
    ) -> Option<(&'a str, &'a String, usize)> {
        let (matched, _, len) = self.district_trie.find_longest_prefix(text)?;
        let names = &self.district_aliases[matched];
        // 原文全名不可被同名简称覆盖，冲突交给输出校验。
        if let Some(full) = names.iter().find(|name| name.as_str() == matched) {
            return Some((matched, full, len));
        }
        let mut candidates: Vec<_> = names
            .iter()
            .filter(|name| {
                self.index.district_to_city[*name].iter().any(|(p, c)| {
                    result.province.as_ref().is_none_or(|known| known == p)
                        && result.city.as_ref().is_none_or(|known| known == c)
                })
            })
            .collect();
        let following: Vec<_> = candidates
            .iter()
            .copied()
            .filter(|name| self.following_parent_matches(name, &text[len..]))
            .collect();
        if !following.is_empty() {
            candidates = following;
        }
        if candidates.len() == 1 {
            Some((matched, candidates[0], len))
        } else {
            None
        }
    }

    // 无上下文的短区名需要地址线索，避免把“合作共赢”识别为合作市。
    fn allow_district(
        &self,
        matched: &str,
        full: &str,
        tail: &str,
        result: &ParsedAddress,
    ) -> bool {
        if matched == full || self.reviewed_district_aliases.contains(matched) {
            return true;
        }
        if is_road_suffix(tail) {
            return false;
        }
        result.province.is_some()
            || result.city.is_some()
            || tail.trim().is_empty()
            || ["路", "街", "巷", "园", "村", "小区"]
                .iter()
                .any(|cue| tail.chars().take(10).collect::<String>().contains(cue))
            || self
                .city_trie
                .find_longest_prefix(tail.trim_start_matches(is_separator))
                .is_some()
            || self
                .province_trie
                .find_longest_prefix(tail.trim_start_matches(is_separator))
                .is_some()
    }

    /// 标准化地址
    ///
    /// 将省、市、区简称转换为标准全称并拼接。
    ///
    /// # 参数
    /// * `province` - 省份（可以是简称）
    /// * `city` - 城市（可以是简称）
    /// * `district` - 区县（可选，可以是简称）
    ///
    /// # 示例
    /// ```rust
    /// use cpca::AddressParser;
    ///
    /// let parser = AddressParser::new();
    /// let full = parser.normalize("广东", "深圳", Some("南山"));
    /// assert_eq!(full, "广东省深圳市南山区");
    /// ```
    pub fn normalize(
        &self,
        province: impl AsRef<str>,
        city: impl AsRef<str>,
        district: Option<&str>,
    ) -> String {
        let province = province.as_ref();
        let city = city.as_ref();

        // 标准化省份
        let norm_province = self
            .province_aliases
            .get(province)
            .map(|s| s.to_string())
            .or_else(|| {
                if self.index.provinces.contains(province) {
                    Some(province.to_string())
                } else {
                    // 尝试添加常见后缀
                    let with_suffix = format!("{}省", province);
                    if self.index.provinces.contains(&with_suffix) {
                        Some(with_suffix)
                    } else {
                        None
                    }
                }
            })
            .unwrap_or_else(|| province.to_string());

        // 标准化城市
        let norm_city = if self.index.cities.contains(city) {
            city.to_string()
        } else {
            let with_suffix = format!("{}市", city);
            if self.index.cities.contains(&with_suffix) {
                with_suffix
            } else {
                city.to_string()
            }
        };

        // 标准化区县
        let norm_district = district.map(|d| {
            if self.index.districts.contains(d) {
                d.to_string()
            } else {
                // 尝试添加常见后缀
                for suffix in &["区", "县", "市"] {
                    let with_suffix = format!("{}{}", d, suffix);
                    if self.index.districts.contains(&with_suffix) {
                        return with_suffix;
                    }
                }
                d.to_string()
            }
        });

        // 拼接
        let mut result = norm_province;
        result.push_str(&norm_city);
        if let Some(d) = norm_district {
            result.push_str(&d);
        }
        result
    }

    /// 批量解析地址
    ///
    /// # 参数
    /// * `addresses` - 地址列表
    ///
    /// # 返回
    /// 解析结果列表
    pub fn parse_batch(&self, addresses: &[&str]) -> Vec<ParsedAddress> {
        addresses.iter().map(|a| self.parse(a)).collect()
    }

    /// 兼容接口：仅判断是否解析出省或市，不代表区划或地址有效。
    ///
    /// 需要区划状态时请使用 [`Self::parse_with_status`] 或 [`Self::validate`]。
    pub fn is_valid_address(&self, address: &str) -> bool {
        let result = self.parse(address);
        result.province.is_some() || result.city.is_some()
    }

    /// 获取所有省份列表
    pub fn provinces(&self) -> Vec<&String> {
        self.index.provinces.iter().collect()
    }

    /// 获取某省份下的所有城市
    pub fn cities_of_province(&self, province: &str) -> Vec<&String> {
        // 尝试标准化省份名
        let norm_province = self
            .province_aliases
            .get(province)
            .map(|s| s.to_string())
            .unwrap_or_else(|| province.to_string());

        self.index
            .province_cities
            .get(&norm_province)
            .map(|cities| cities.iter().collect())
            .unwrap_or_default()
    }

    /// 获取某城市下的所有区县
    pub fn districts_of_city(&self, city: &str) -> Vec<&String> {
        // 尝试标准化城市名
        let norm_city = if self.index.cities.contains(city) {
            city.to_string()
        } else {
            format!("{}市", city)
        };

        self.index
            .city_districts
            .get(&norm_city)
            .map(|districts| districts.iter().collect())
            .unwrap_or_default()
    }
}

impl Default for AddressParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parser() -> AddressParser {
        AddressParser::new()
    }

    // ==================== 基本功能测试 ====================

    #[test]
    fn test_parse_full_address() {
        let p = parser();
        let r = p.parse("广东省深圳市南山区科技园路1号");

        assert_eq!(r.province, Some("广东省".to_string()));
        assert_eq!(r.city, Some("深圳市".to_string()));
        assert_eq!(r.district, Some("南山区".to_string()));
        assert_eq!(r.detail, "科技园路1号");
    }

    #[test]
    fn test_parse_with_short_province() {
        let p = parser();
        let r = p.parse("广东深圳市南山区");

        assert_eq!(r.province, Some("广东省".to_string()));
        assert_eq!(r.city, Some("深圳市".to_string()));
        assert_eq!(r.district, Some("南山区".to_string()));
    }

    #[test]
    fn test_parse_with_short_city() {
        let p = parser();
        let r = p.parse("广东省深圳南山区");

        assert_eq!(r.province, Some("广东省".to_string()));
        assert_eq!(r.city, Some("深圳市".to_string()));
        assert_eq!(r.district, Some("南山区".to_string()));
    }

    #[test]
    fn test_parse_with_short_district() {
        let p = parser();
        let r = p.parse("广东省深圳市南山");

        assert_eq!(r.province, Some("广东省".to_string()));
        assert_eq!(r.city, Some("深圳市".to_string()));
        assert_eq!(r.district, Some("南山区".to_string()));
    }

    // ==================== 缺省情况测试 ====================

    #[test]
    fn test_parse_no_province() {
        // 只有市+区，自动推断省份
        let p = parser();
        let r = p.parse("深圳市南山区科技园");

        assert_eq!(r.province, Some("广东省".to_string()));
        assert_eq!(r.city, Some("深圳市".to_string()));
        assert_eq!(r.district, Some("南山区".to_string()));
    }

    #[test]
    fn test_parse_no_province_short_city() {
        // 只有市（简称）+区
        let p = parser();
        let r = p.parse("深圳南山区科技园");

        assert_eq!(r.province, Some("广东省".to_string()));
        assert_eq!(r.city, Some("深圳市".to_string()));
        assert_eq!(r.district, Some("南山区".to_string()));
    }

    #[test]
    fn test_parse_only_city() {
        // 只有城市
        let p = parser();
        let r = p.parse("深圳市某某路");

        assert_eq!(r.province, Some("广东省".to_string()));
        assert_eq!(r.city, Some("深圳市".to_string()));
        assert_eq!(r.district, None);
        assert_eq!(r.detail, "某某路");
    }

    #[test]
    fn test_parse_only_district() {
        // 只有区县（如果区名唯一）
        let p = parser();
        let r = p.parse("南山区科技园");

        // 南山区可能不唯一，取决于数据
        // 但至少应该能识别到区
        assert!(r.district.is_some() || r.detail.contains("南山"));
    }

    #[test]
    fn test_parse_province_and_district_no_city() {
        // 省+区，没有市
        let p = parser();
        let r = p.parse("广东省南山区");

        assert_eq!(r.province, Some("广东省".to_string()));
        // 应该能推断出城市
        if r.district == Some("南山区".to_string()) {
            assert_eq!(r.city, Some("深圳市".to_string()));
        }
    }

    // ==================== 直辖市测试 ====================

    #[test]
    fn test_parse_municipality_full() {
        let p = parser();
        let r = p.parse("北京市朝阳区望京");

        assert_eq!(r.province, Some("北京市".to_string()));
        assert_eq!(r.city, Some("北京市".to_string()));
        assert_eq!(r.district, Some("朝阳区".to_string()));
        assert_eq!(r.detail, "望京");
    }

    #[test]
    fn test_parse_municipality_short() {
        let p = parser();
        let r = p.parse("北京朝阳区");

        assert_eq!(r.province, Some("北京市".to_string()));
        assert_eq!(r.city, Some("北京市".to_string()));
        assert_eq!(r.district, Some("朝阳区".to_string()));
    }

    #[test]
    fn test_parse_shanghai() {
        let p = parser();
        let r = p.parse("上海市浦东新区陆家嘴");

        assert_eq!(r.province, Some("上海市".to_string()));
        assert_eq!(r.city, Some("上海市".to_string()));
        assert_eq!(r.district, Some("浦东新区".to_string()));
    }

    #[test]
    fn test_parse_chongqing() {
        let p = parser();
        let r = p.parse("重庆市渝中区解放碑");

        assert_eq!(r.province, Some("重庆市".to_string()));
        assert_eq!(r.city, Some("重庆市".to_string()));
        assert_eq!(r.district, Some("渝中区".to_string()));
    }

    // ==================== 自治区测试 ====================

    #[test]
    fn test_parse_autonomous_region() {
        let p = parser();
        let r = p.parse("广西壮族自治区南宁市青秀区");

        assert_eq!(r.province, Some("广西壮族自治区".to_string()));
        assert_eq!(r.city, Some("南宁市".to_string()));
        assert_eq!(r.district, Some("青秀区".to_string()));
    }

    #[test]
    fn test_parse_autonomous_region_short() {
        let p = parser();
        let r = p.parse("广西南宁市");

        assert_eq!(r.province, Some("广西壮族自治区".to_string()));
        assert_eq!(r.city, Some("南宁市".to_string()));
    }

    #[test]
    fn test_parse_inner_mongolia() {
        let p = parser();
        let r = p.parse("内蒙古自治区呼和浩特市");

        assert_eq!(r.province, Some("内蒙古自治区".to_string()));
        assert_eq!(r.city, Some("呼和浩特市".to_string()));
    }

    #[test]
    fn test_parse_inner_mongolia_short() {
        let p = parser();
        let r = p.parse("内蒙古呼和浩特");

        assert_eq!(r.province, Some("内蒙古自治区".to_string()));
        assert_eq!(r.city, Some("呼和浩特市".to_string()));
    }

    // ==================== 不设区的市测试 ====================

    #[test]
    fn test_parse_dongguan() {
        // 东莞市没有区，直接是镇
        let p = parser();
        let r = p.parse("广东省东莞市长安镇");

        assert_eq!(r.province, Some("广东省".to_string()));
        assert_eq!(r.city, Some("东莞市".to_string()));
        // 长安镇可能在 district 或 detail 中
    }

    #[test]
    fn test_parse_zhongshan() {
        let p = parser();
        let r = p.parse("广东省中山市小榄镇");

        assert_eq!(r.province, Some("广东省".to_string()));
        assert_eq!(r.city, Some("中山市".to_string()));
    }

    // ==================== 自治州测试 ====================

    #[test]
    fn test_parse_autonomous_prefecture() {
        let p = parser();
        let r = p.parse("云南省大理白族自治州大理市");

        assert_eq!(r.province, Some("云南省".to_string()));
        assert_eq!(r.city, Some("大理白族自治州".to_string()));
        assert_eq!(r.district, Some("大理市".to_string()));
    }

    // ==================== 边界情况测试 ====================

    #[test]
    fn test_parse_empty() {
        let p = parser();
        let r = p.parse("");

        assert_eq!(r.province, None);
        assert_eq!(r.city, None);
        assert_eq!(r.district, None);
        assert_eq!(r.detail, "");
    }

    #[test]
    fn test_parse_whitespace() {
        let p = parser();
        let r = p.parse("   ");

        assert_eq!(r.province, None);
        assert_eq!(r.city, None);
        assert_eq!(r.district, None);
    }

    #[test]
    fn test_parse_only_detail() {
        let p = parser();
        let r = p.parse("某某路123号");

        assert_eq!(r.province, None);
        assert_eq!(r.city, None);
        assert_eq!(r.district, None);
        assert_eq!(r.detail, "某某路123号");
    }

    #[test]
    fn test_parse_with_extra_spaces() {
        let p = parser();
        let r = p.parse("  广东省  深圳市  南山区  ");

        assert_eq!(r.province, Some("广东省".to_string()));
        // 注意：中间的空格会影响匹配，这是预期行为
    }

    // ==================== 标准化测试 ====================

    #[test]
    fn test_normalize_full() {
        let p = parser();
        let result = p.normalize("广东省", "深圳市", Some("南山区"));
        assert_eq!(result, "广东省深圳市南山区");
    }

    #[test]
    fn test_normalize_short_names() {
        let p = parser();
        let result = p.normalize("广东", "深圳", Some("南山"));
        assert_eq!(result, "广东省深圳市南山区");
    }

    #[test]
    fn test_normalize_no_district() {
        let p = parser();
        let result = p.normalize("广东", "深圳", None);
        assert_eq!(result, "广东省深圳市");
    }

    #[test]
    fn test_normalize_municipality() {
        let p = parser();
        let result = p.normalize("北京", "北京", Some("朝阳"));
        assert_eq!(result, "北京市北京市朝阳区");
    }

    // ==================== 批量处理测试 ====================

    #[test]
    fn test_parse_batch() {
        let p = parser();
        let addresses = vec!["广东省深圳市南山区", "北京市朝阳区", "上海市浦东新区"];
        let results = p.parse_batch(&addresses);

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].province, Some("广东省".to_string()));
        assert_eq!(results[1].province, Some("北京市".to_string()));
        assert_eq!(results[2].province, Some("上海市".to_string()));
    }

    // ==================== 辅助方法测试 ====================

    #[test]
    fn test_is_valid_address() {
        let p = parser();

        assert!(p.is_valid_address("广东省深圳市"));
        assert!(p.is_valid_address("深圳市"));
        assert!(!p.is_valid_address("某某路123号"));
        assert!(!p.is_valid_address(""));
    }

    #[test]
    fn test_provinces_list() {
        let p = parser();
        let provinces = p.provinces();

        assert!(!provinces.is_empty());
        assert!(provinces.iter().any(|p| *p == "广东省"));
        assert!(provinces.iter().any(|p| *p == "北京市"));
    }

    #[test]
    fn test_cities_of_province() {
        let p = parser();
        let cities = p.cities_of_province("广东省");

        assert!(!cities.is_empty());
        assert!(cities.iter().any(|c| *c == "深圳市"));
        assert!(cities.iter().any(|c| *c == "广州市"));
    }

    #[test]
    fn test_districts_of_city() {
        let p = parser();
        let districts = p.districts_of_city("深圳市");

        assert!(!districts.is_empty());
        assert!(districts.iter().any(|d| *d == "南山区"));
        assert!(districts.iter().any(|d| *d == "福田区"));
    }

    // ==================== 同名地区测试 ====================

    #[test]
    fn test_parse_duplicate_district_name() {
        // 朝阳区在北京和长春都有
        let p = parser();

        // 有上下文时应该能正确识别
        let r1 = p.parse("北京市朝阳区");
        assert_eq!(r1.province, Some("北京市".to_string()));
        assert_eq!(r1.district, Some("朝阳区".to_string()));

        let r2 = p.parse("吉林省长春市朝阳区");
        assert_eq!(r2.province, Some("吉林省".to_string()));
        assert_eq!(r2.city, Some("长春市".to_string()));
        assert_eq!(r2.district, Some("朝阳区".to_string()));
    }

    // ==================== 全局解析器测试 ====================

    #[test]
    fn test_global_parser() {
        let r = crate::parse("广东省深圳市");
        assert_eq!(r.province, Some("广东省".to_string()));
        assert_eq!(r.city, Some("深圳市".to_string()));
    }

    #[test]
    fn test_global_normalize() {
        let result = crate::normalize("广东", "深圳", Some("南山"));
        assert_eq!(result, "广东省深圳市南山区");
    }

    // ==================== 自治州简称测试 ====================

    #[test]
    fn test_parse_autonomous_prefecture_short() {
        let p = parser();

        // 省+自治州简称
        let r = p.parse("云南大理");
        assert_eq!(r.province, Some("云南省".to_string()));
        assert_eq!(r.city, Some("大理白族自治州".to_string()));

        let r = p.parse("四川甘孜");
        assert_eq!(r.province, Some("四川省".to_string()));
        assert_eq!(r.city, Some("甘孜藏族自治州".to_string()));

        // 省+县级市
        let r = p.parse("四川康定");
        assert_eq!(r.province, Some("四川省".to_string()));
        assert_eq!(r.city, Some("甘孜藏族自治州".to_string()));
        assert_eq!(r.district, Some("康定市".to_string()));
    }

    // ==================== 县级市测试 ====================

    #[test]
    fn test_parse_county_level_city() {
        let p = parser();

        // 只给县级市名
        let r = p.parse("康定市");
        assert_eq!(r.province, Some("四川省".to_string()));
        assert_eq!(r.city, Some("甘孜藏族自治州".to_string()));
        assert_eq!(r.district, Some("康定市".to_string()));

        let r = p.parse("大理市");
        assert_eq!(r.province, Some("云南省".to_string()));
        assert_eq!(r.city, Some("大理白族自治州".to_string()));
        assert_eq!(r.district, Some("大理市".to_string()));

        let r = p.parse("义乌市");
        assert_eq!(r.province, Some("浙江省".to_string()));
        assert_eq!(r.city, Some("金华市".to_string()));
        assert_eq!(r.district, Some("义乌市".to_string()));

        let r = p.parse("昆山市");
        assert_eq!(r.province, Some("江苏省".to_string()));
        assert_eq!(r.city, Some("苏州市".to_string()));
        assert_eq!(r.district, Some("昆山市".to_string()));

        let r = p.parse("寿光市");
        assert_eq!(r.province, Some("山东省".to_string()));
        assert_eq!(r.city, Some("潍坊市".to_string()));
        assert_eq!(r.district, Some("寿光市".to_string()));
    }

    // ==================== 边界情况测试 ====================

    #[test]
    fn test_parse_ambiguous_district() {
        let p = parser();

        // 南山区在多个城市都有，无上下文时无法确定城市
        let r = p.parse("南山区");
        assert!(r.district.is_some()); // 能识别区
                                       // 没有足够上下文，可能无法确定城市

        // 有上下文时能正确识别
        let r = p.parse("深圳南山区");
        assert_eq!(r.province, Some("广东省".to_string()));
        assert_eq!(r.city, Some("深圳市".to_string()));
        assert_eq!(r.district, Some("南山区".to_string()));
    }

    #[test]
    fn test_parse_city_district_same_name() {
        // 朝阳既是辽宁的地级市，也是北京/长春的区
        let p = parser();

        // 明确指定北京
        let r = p.parse("北京朝阳");
        assert_eq!(r.province, Some("北京市".to_string()));
        assert_eq!(r.city, Some("北京市".to_string()));
        assert_eq!(r.district, Some("朝阳区".to_string()));

        // 明确指定长春
        let r = p.parse("长春朝阳区");
        assert_eq!(r.province, Some("吉林省".to_string()));
        assert_eq!(r.city, Some("长春市".to_string()));
        assert_eq!(r.district, Some("朝阳区".to_string()));
    }

    // ==================== 全量匹配优先测试 ====================

    #[test]
    fn test_full_match_priority() {
        // 关键测试：朝阳区 应该匹配为区县，而不是朝阳市
        let p = parser();

        // "朝阳区" 应该识别为区县，而不是被解析成 "朝阳市"
        let r = p.parse("朝阳区");
        assert_eq!(r.district, Some("朝阳区".to_string()));
        // 由于朝阳区在多个城市都有，不指定上下文时不应该推断城市
        // 但绝对不应该被匹配成朝阳市

        // 带上下文的情况
        let r = p.parse("北京朝阳区");
        assert_eq!(r.province, Some("北京市".to_string()));
        assert_eq!(r.city, Some("北京市".to_string()));
        assert_eq!(r.district, Some("朝阳区".to_string()));

        // 辽宁朝阳市的情况 - 应该正确匹配为城市
        let r = p.parse("辽宁朝阳");
        assert_eq!(r.province, Some("辽宁省".to_string()));
        assert_eq!(r.city, Some("朝阳市".to_string()));

        let r = p.parse("辽宁省朝阳市");
        assert_eq!(r.province, Some("辽宁省".to_string()));
        assert_eq!(r.city, Some("朝阳市".to_string()));
    }

    #[test]
    fn test_district_suffix_priority() {
        // 带有明确后缀的区县应该优先匹配
        let p = parser();

        // 福田区 - 应该匹配为区县
        let r = p.parse("福田区");
        assert_eq!(r.district, Some("福田区".to_string()));

        // 南山区 - 应该匹配为区县
        let r = p.parse("南山区");
        assert_eq!(r.district, Some("南山区".to_string()));

        // 宝安区 - 应该匹配为区县
        let r = p.parse("宝安区");
        assert_eq!(r.district, Some("宝安区".to_string()));
    }
}
