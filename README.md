# CPCA - Chinese Province City Area Parser

[![Crates.io](https://img.shields.io/crates/v/cpca.svg)](https://crates.io/crates/cpca)
[![Documentation](https://docs.rs/cpca/badge.svg)](https://docs.rs/cpca)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

中国省市区地址解析库，用于从自由文本地址中提取省、市、区信息。

## 功能特性

- 🚀 **高性能** - 基于前缀树实现，O(n) 时间复杂度
- 📦 **零依赖** - 纯 Rust 实现，无需外部分词库
- 🎯 **智能匹配** - 支持省份简称、城市简称、区县简称
- 🏛️ **完整数据** - 内置 2025 年最新行政区划（3600+ 条记录）
- 🔧 **易于使用** - 简洁的 API，开箱即用

## 安装

```toml
[dependencies]
cpca = "0.1"
```

## 快速开始

```rust
use cpca::AddressParser;

fn main() {
    let parser = AddressParser::new();

    // 解析完整地址
    let result = parser.parse("广东省深圳市南山区科技园路1号");
    println!("省份: {:?}", result.province);  // Some("广东省")
    println!("城市: {:?}", result.city);      // Some("深圳市")
    println!("区县: {:?}", result.district);  // Some("南山区")
    println!("详址: {}", result.detail);      // "科技园路1号"

    // 支持简称
    let result = parser.parse("深圳南山科技园");
    assert_eq!(result.province, Some("广东省".to_string()));
    assert_eq!(result.city, Some("深圳市".to_string()));
    assert_eq!(result.district, Some("南山区".to_string()));
}
```

## 便捷函数

```rust
// 使用全局解析器（无需手动创建实例）
let result = cpca::parse("北京市朝阳区望京");

// 标准化地址（简称转全称）
let full = cpca::normalize("广东", "深圳", Some("南山"));
assert_eq!(full, "广东省深圳市南山区");
```

## 支持的场景

### 完整地址
```rust
parser.parse("广东省深圳市南山区科技园");
// 省份: 广东省, 城市: 深圳市, 区县: 南山区
```

### 省份简称
```rust
parser.parse("广东深圳市南山区");
// 自动识别 "广东" -> "广东省"
```

### 缺省省份
```rust
parser.parse("深圳市南山区科技园");
// 自动推断省份为 "广东省"
```

### 直辖市
```rust
parser.parse("北京市朝阳区");
// 省份: 北京市, 城市: 北京市, 区县: 朝阳区
```

### 自治区
```rust
parser.parse("广西南宁市");
// 自动识别 "广西" -> "广西壮族自治区"
```

### 自治州
```rust
parser.parse("云南省大理白族自治州大理市");
// 正确识别自治州级城市
```

## API 文档

### AddressParser

```rust
impl AddressParser {
    /// 创建新的解析器实例
    fn new() -> Self;

    /// 获取全局单例
    fn global() -> &'static AddressParser;

    /// 解析地址
    fn parse(&self, address: &str) -> ParsedAddress;

    /// 标准化地址
    fn normalize(&self, province: &str, city: &str, district: Option<&str>) -> String;

    /// 批量解析
    fn parse_batch(&self, addresses: &[&str]) -> Vec<ParsedAddress>;

    /// 验证地址有效性
    fn is_valid_address(&self, address: &str) -> bool;

    /// 获取所有省份
    fn provinces(&self) -> Vec<&String>;

    /// 获取某省的所有城市
    fn cities_of_province(&self, province: &str) -> Vec<&String>;

    /// 获取某市的所有区县
    fn districts_of_city(&self, city: &str) -> Vec<&String>;
}
```

### ParsedAddress

```rust
pub struct ParsedAddress {
    pub province: Option<String>,  // 省份
    pub city: Option<String>,      // 城市
    pub district: Option<String>,  // 区县
    pub detail: String,            // 详细地址
}

impl ParsedAddress {
    fn is_complete(&self) -> bool;   // 是否完整（省市区都有）
    fn full_address(&self) -> String; // 拼接完整地址
}
```

## 特性 (Features)

- `serde` - 启用 serde 序列化支持

```toml
[dependencies]
cpca = { version = "0.1", features = ["serde"] }
```

## 数据来源

行政区划数据来自 [AreaCity-JsSpider-StatsGov](https://github.com/xiangyuecn/AreaCity-JsSpider-StatsGov)，包含：

- 34 个省级行政区（含港澳台）
- 300+ 个地级市/自治州
- 2800+ 个区县

数据更新至 2025 年。

## 性能

在 M1 Mac 上的基准测试结果：

| 操作 | 耗时 |
|------|------|
| 解析完整地址 | ~500ns |
| 解析简称地址 | ~600ns |
| 标准化地址 | ~200ns |

## 与 Python cpca 的对比

| 特性 | cpca (Python) | cpca (Rust) |
|------|---------------|-------------|
| 分词依赖 | jieba | 无需 |
| 性能 | 较慢 | 快 10-50x |
| 内存 | ~50MB | ~5MB |
| 部署 | 需要 Python | 单一二进制 |

## License

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！
