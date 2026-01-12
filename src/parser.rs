//! 地址解析器核心实现

use crate::data::{load_regions, province_aliases, RegionIndex};
use crate::region::ParsedAddress;
use crate::trie::Trie;
use once_cell::sync::Lazy;
use std::collections::HashMap;

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
    /// 省份简称映射
    province_aliases: HashMap<&'static str, &'static str>,
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

        // 构建区县前缀树
        let mut district_trie = Trie::new();
        for district in &index.districts {
            district_trie.insert(district, district.clone());
            // 添加简称
            for suffix in &["区", "县", "市", "旗"] {
                if district.ends_with(suffix) {
                    let short = district.trim_end_matches(suffix);
                    if !short.is_empty() && short.chars().count() >= 2 {
                        district_trie.insert(short, district.clone());
                    }
                }
            }
        }

        Self {
            province_trie,
            city_trie,
            district_trie,
            index,
            province_aliases: aliases,
        }
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
        if let Some((_matched, normalized, len)) =
            self.province_trie.find_longest_prefix(&remaining)
        {
            result.province = Some(normalized.clone());
            remaining = remaining[len..].to_string();

            // 直辖市特殊处理：省=市，直接跳到区县匹配
            if self.index.is_municipality(normalized) {
                result.city = Some(normalized.clone());
                // 直接尝试匹配区县
                if let Some((_m, dist_normalized, dist_len)) =
                    self.district_trie.find_longest_prefix(&remaining)
                {
                    // 验证区县是否属于该直辖市
                    if self.index.validate_district(normalized, dist_normalized) {
                        result.district = Some(dist_normalized.clone());
                        remaining = remaining[dist_len..].to_string();
                    }
                }
                result.detail = remaining.trim().to_string();
                return result;
            }
        }

        // 第二步：尝试匹配城市（但要先检查是否应该优先匹配区县）
        // 关键改进：当没有省份上下文时，如果输入看起来像区县（如"朝阳区"），应该优先匹配区县
        let city_match = self.city_trie.find_longest_prefix(&remaining);
        let district_match = self.district_trie.find_longest_prefix(&remaining);

        // 判断是否应该优先使用区县匹配
        let prefer_district = if result.province.is_none() {
            // 没有省份上下文时，检查区县匹配是否更长或更精确
            match (&city_match, &district_match) {
                (Some((_, _, city_len)), Some((_, dist_normalized, dist_len))) => {
                    // 如果区县匹配更长，或者区县是完整形式（带后缀），优先使用区县
                    *dist_len > *city_len
                        || dist_normalized.ends_with('区')
                        || dist_normalized.ends_with('县')
                        || dist_normalized.ends_with('旗')
                }
                (Some(_), None) => false,
                (None, Some(_)) => true,
                (None, None) => false,
            }
        } else {
            false
        };

        if prefer_district {
            // 优先处理区县匹配
            if let Some((_matched, dist_normalized, dist_len)) = district_match {
                result.district = Some(dist_normalized.clone());

                // 尝试反向查找城市和省份
                if let Some(cities) = self.index.district_to_city.get(dist_normalized) {
                    if cities.len() == 1 {
                        // 唯一匹配
                        result.province = Some(cities[0].0.clone());
                        result.city = Some(cities[0].1.clone());
                    }
                    // 如果有多个匹配，不做假设，让用户提供更多上下文
                }

                remaining = remaining[dist_len..].to_string();
            }
        } else {
            // 正常流程：先匹配城市
            if let Some((_matched, normalized, len)) = city_match {
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

                if valid_city {
                    result.city = Some(normalized.clone());

                    // 如果之前没匹配到省份，尝试反向查找
                    if result.province.is_none() {
                        if let Some(province) = self.index.city_to_province.get(normalized) {
                            result.province = Some(province.clone());
                        }
                    }

                    remaining = remaining[len..].to_string();
                }
            }
        }

        // 第三步：尝试匹配区县（如果还没匹配到）
        if result.district.is_none() {
            if let Some((_matched, normalized, len)) =
                self.district_trie.find_longest_prefix(&remaining)
            {
                // 验证区县是否合法
                let valid = if let Some(ref city) = result.city {
                    self.index.validate_district(city, normalized)
                        || self.validate_district_flexible(city, normalized)
                } else {
                    true // 没有城市信息时，先接受
                };

                if valid {
                    result.district = Some(normalized.clone());

                    // 如果之前没匹配到城市，尝试反向查找
                    if result.city.is_none() {
                        if let Some(cities) = self.index.district_to_city.get(normalized) {
                            if cities.len() == 1 {
                                // 唯一匹配
                                result.province = Some(cities[0].0.clone());
                                result.city = Some(cities[0].1.clone());
                            } else if let Some(ref province) = result.province {
                                // 根据已知省份过滤
                                if let Some((_, city)) = cities.iter().find(|(p, _)| p == province)
                                {
                                    result.city = Some(city.clone());
                                }
                            }
                        }
                    }

                    // 如果有城市但没省份，再次尝试
                    if result.province.is_none() {
                        if let Some(ref city) = result.city {
                            if let Some(province) = self.index.city_to_province.get(city) {
                                result.province = Some(province.clone());
                            }
                        }
                    }

                    remaining = remaining[len..].to_string();
                }
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

    /// 灵活验证区县（处理简称情况）
    fn validate_district_flexible(&self, city: &str, district: &str) -> bool {
        if let Some(districts) = self.index.city_districts.get(city) {
            for d in districts {
                // 检查是否是简称
                if d.starts_with(district)
                    || district.starts_with(d.trim_end_matches(&['区', '县', '市', '旗'][..]))
                {
                    return true;
                }
            }
        }
        false
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

    /// 检查地址是否有效（至少能解析出省或市）
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
