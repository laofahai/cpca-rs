//! 地区数据结构

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// 行政区划记录
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Region {
    /// 省份（含直辖市、自治区）
    pub province: String,
    /// 城市（地级市、自治州等）
    pub city: String,
    /// 区县（可能为空，如不设区的市）
    pub district: Option<String>,
}

impl Region {
    /// 创建新的行政区划记录
    pub fn new(province: impl Into<String>, city: impl Into<String>, district: Option<String>) -> Self {
        Self {
            province: province.into(),
            city: city.into(),
            district,
        }
    }

    /// 获取完整地址字符串
    pub fn full_name(&self) -> String {
        match &self.district {
            Some(d) => format!("{}{}{}", self.province, self.city, d),
            None => format!("{}{}", self.province, self.city),
        }
    }
}

/// 解析结果
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ParsedAddress {
    /// 省份
    pub province: Option<String>,
    /// 城市
    pub city: Option<String>,
    /// 区县
    pub district: Option<String>,
    /// 剩余详细地址
    pub detail: String,
}

impl ParsedAddress {
    /// 创建空的解析结果
    pub fn empty() -> Self {
        Self::default()
    }

    /// 是否解析到了省份
    pub fn has_province(&self) -> bool {
        self.province.is_some()
    }

    /// 是否解析到了城市
    pub fn has_city(&self) -> bool {
        self.city.is_some()
    }

    /// 是否解析到了区县
    pub fn has_district(&self) -> bool {
        self.district.is_some()
    }

    /// 是否完整解析（省市区都有）
    pub fn is_complete(&self) -> bool {
        self.province.is_some() && self.city.is_some() && self.district.is_some()
    }

    /// 获取标准化的完整地址
    pub fn full_address(&self) -> String {
        let mut result = String::new();
        if let Some(ref p) = self.province {
            result.push_str(p);
        }
        if let Some(ref c) = self.city {
            // 避免直辖市重复
            if self.province.as_ref() != Some(c) {
                result.push_str(c);
            }
        }
        if let Some(ref d) = self.district {
            result.push_str(d);
        }
        if !self.detail.is_empty() {
            result.push_str(&self.detail);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_full_name() {
        let region = Region::new("广东省", "深圳市", Some("南山区".to_string()));
        assert_eq!(region.full_name(), "广东省深圳市南山区");

        let region = Region::new("广东省", "东莞市", None);
        assert_eq!(region.full_name(), "广东省东莞市");
    }

    #[test]
    fn test_parsed_address() {
        let addr = ParsedAddress {
            province: Some("广东省".to_string()),
            city: Some("深圳市".to_string()),
            district: Some("南山区".to_string()),
            detail: "科技园".to_string(),
        };
        assert!(addr.is_complete());
        assert_eq!(addr.full_address(), "广东省深圳市南山区科技园");
    }

    #[test]
    fn test_municipality_address() {
        let addr = ParsedAddress {
            province: Some("北京市".to_string()),
            city: Some("北京市".to_string()),
            district: Some("朝阳区".to_string()),
            detail: "望京".to_string(),
        };
        // 直辖市不重复显示
        assert_eq!(addr.full_address(), "北京市朝阳区望京");
    }
}
