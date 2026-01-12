//! # CPCA - Chinese Province City Area Parser
//!
//! 中国省市区地址解析库，从地址字符串中提取省、市、区信息。
//!
//! ## 功能特性
//!
//! - 从自由文本地址中提取省、市、区
//! - 支持省份简称（如 "广东" -> "广东省"）
//! - 支持直辖市特殊处理
//! - 支持不设区的地级市（东莞、中山、儋州、嘉峪关）
//! - 内置 2025 年最新行政区划数据（3600+ 条记录）
//!
//! ## 快速开始
//!
//! ```rust
//! use cpca::AddressParser;
//!
//! let parser = AddressParser::new();
//!
//! // 解析完整地址
//! let result = parser.parse("广东省深圳市南山区科技园");
//! assert_eq!(result.province, Some("广东省".to_string()));
//! assert_eq!(result.city, Some("深圳市".to_string()));
//! assert_eq!(result.district, Some("南山区".to_string()));
//!
//! // 支持简称
//! let result = parser.parse("深圳南山科技园");
//! assert_eq!(result.province, Some("广东省".to_string()));
//! assert_eq!(result.city, Some("深圳市".to_string()));
//!
//! // 标准化地址
//! let full = parser.normalize("广东", "深圳", None);
//! assert_eq!(full, "广东省深圳市");
//! ```

mod data;
mod error;
mod parser;
mod region;
mod trie;

pub use error::ParseError;
pub use parser::AddressParser;
pub use region::{ParsedAddress, Region};

/// 便捷函数：使用全局解析器解析地址
///
/// ```rust
/// let result = cpca::parse("北京市朝阳区");
/// assert_eq!(result.province, Some("北京市".to_string()));
/// ```
pub fn parse(address: &str) -> ParsedAddress {
    AddressParser::global().parse(address)
}

/// 便捷函数：标准化地址
///
/// ```rust
/// let full = cpca::normalize("广东", "深圳", Some("南山"));
/// assert_eq!(full, "广东省深圳市南山区");
/// ```
pub fn normalize(
    province: impl AsRef<str>,
    city: impl AsRef<str>,
    district: Option<&str>,
) -> String {
    AddressParser::global().normalize(province, city, district)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full_address() {
        let parser = AddressParser::new();
        let result = parser.parse("广东省深圳市南山区科技园路1号");

        assert_eq!(result.province, Some("广东省".to_string()));
        assert_eq!(result.city, Some("深圳市".to_string()));
        assert_eq!(result.district, Some("南山区".to_string()));
        assert_eq!(result.detail, "科技园路1号");
    }

    #[test]
    fn test_parse_short_name() {
        let parser = AddressParser::new();
        let result = parser.parse("深圳南山科技园");

        assert_eq!(result.province, Some("广东省".to_string()));
        assert_eq!(result.city, Some("深圳市".to_string()));
        assert_eq!(result.district, Some("南山区".to_string()));
    }

    #[test]
    fn test_parse_municipality() {
        let parser = AddressParser::new();
        let result = parser.parse("北京市朝阳区望京");

        assert_eq!(result.province, Some("北京市".to_string()));
        assert_eq!(result.city, Some("北京市".to_string()));
        assert_eq!(result.district, Some("朝阳区".to_string()));
    }

    #[test]
    fn test_normalize() {
        let parser = AddressParser::new();

        assert_eq!(
            parser.normalize("广东", "深圳", Some("南山")),
            "广东省深圳市南山区"
        );
        assert_eq!(parser.normalize("广东省", "深圳市", None), "广东省深圳市");
        assert_eq!(
            parser.normalize("北京", "北京", Some("朝阳")),
            "北京市北京市朝阳区"
        );
    }
}
