# cpca — Rust 中国省市区地址解析

从中文地址前缀中提取省、市、区／镇街，支持部分简称、历史名称识别和本地区划组合校验。

[Crates.io](https://crates.io/crates/cpca) · [API 文档](https://docs.rs/cpca) · [Python 版本](https://github.com/laofahai/cpca-linch)

## 安装与快速开始

```toml
[dependencies]
cpca = "0.3"
```

```rust
use cpca::{parse_with_status, RecognitionStatus};

let result = parse_with_status("中山市南朗镇人民路1号");
assert_eq!(result.address.city.as_deref(), Some("中山市"));
assert_eq!(result.address.district.as_deref(), Some("南朗镇"));
assert_eq!(result.address.detail, "人民路1号");
assert_eq!(result.validation.status, RecognitionStatus::Historical);
assert_eq!(result.validation.current_name.as_deref(), Some("南朗街道"));
```

旧名称仍可识别并保留原名，现行名称建议单独返回。旧接口 `cpca::parse()` 和 `ParsedAddress` 的字段结构保持不变。

## 输出与校验

`parse_with_status()` 返回 `CheckedAddress`，包含两个部分：

| 字段 | 内容 |
| --- | --- |
| `address.province`、`city`、`district` | 识别或补全的名称，缺失为 `None` |
| `address.detail` | 剩余详细地址，沿用 Rust 版去掉首尾空白的行为 |
| `validation.status` | 下表中的识别状态 |
| `validation.note` | 变更说明或需要核对的原因 |
| `validation.current_name` | 有明确依据时提供现行名称建议，否则为 `None` |
| `validation.candidates` | 歧义时的候选归属，按省、市、区排序 |

| 状态 | 含义 |
| --- | --- |
| `Current` | 完整组合命中本项目现行快照 |
| `Historical` | 已核验的旧行政名称 |
| `HistoricalManagement` | 旧管理区域称呼，不是现行县级行政区 |
| `Place` | 农场、园区等地点名称 |
| `NeedsReview` | 来源待核验，或已识别的层级互相矛盾 |
| `Ambiguous` | 仍有多个归属候选，不能任意选一个城市 |
| `Incomplete` | 信息不足，不能判断完整区划状态 |

解析时使用表中的父子关系约束候选，输出校验核对整个省市区组合。数据编译进库，无须逐条联网。
**`Current` 只表示组合命中本项目快照，不证明原地址真实有效，也不能替代实时官方校验。**

```rust
use cpca::{parse_with_status, RecognitionStatus};

let conflict = parse_with_status("广东省雁塔区人民路1号");
assert_eq!(conflict.address.province.as_deref(), Some("广东省"));
assert_eq!(conflict.validation.status, RecognitionStatus::NeedsReview);
// 不会为凑出合法组合，将原文中的广东省改为陕西省。

let ambiguous = parse_with_status("江苏省鼓楼区人民路1号");
assert_eq!(ambiguous.address.city, None);
assert_eq!(ambiguous.validation.status, RecognitionStatus::Ambiguous);
// candidates 中包含南京市、徐州市；请补充城市上下文。
```

`is_valid_address()` 是旧版兼容接口，仅判断是否解析出省或市，**不代表区划或地址有效**。
需要状态判断时，使用 `parse_with_status()`；已有 `ParsedAddress` 可传给 `AddressParser::validate()`。
`is_complete()` 同样只判断三个字段是否都有值。

## 常用接口

```rust
use cpca::AddressParser;

let parser = AddressParser::global(); // 复用全局解析器，也可调用 new() 创建实例。
let address = parser.parse("深圳南山科技园");
assert_eq!(address.city.as_deref(), Some("深圳市"));
assert_eq!(address.district.as_deref(), Some("南山区"));

let validation = parser.validate(&address);
let batch = parser.parse_batch(&["北京市朝阳区望京", "中山市沙溪镇"]);
let cities = parser.cities_of_province("广东省");
let districts = parser.districts_of_city("深圳市");
```

`normalize()` 仅进行名称补全和拼接，不执行归属校验，不自动将历史名称换算为现行区划。

```rust
assert_eq!(cpca::normalize("广东", "深圳", Some("南山")), "广东省深圳市南山区");
```

启用序列化支持：

```toml
[dependencies]
cpca = { version = "0.3", features = ["serde"] }
```

原 `ParsedAddress` 的 serde 字段不变；新增校验类型也支持序列化，状态使用上述英文枚举名。

## 匹配范围

支持正序地址，以及部分区在市／省前的写法，例如 `朝阳 北京市望京1号`、`西安市陕西雁塔区人民路1号`。
逆序父级简称需要有边界；“吉林大学”“山东大厦”中的地名不会因此被删掉。

本库主要读取地址前缀，不是任意正文的全文抽取器；不提供多地点抽取、建筑物反查、经纬度或全国历史辖区换算。
与普通词重合的简称采取保守匹配，识别不全时请补充行政名称全称和上级信息。

## 数据范围

| 数据表 | 记录数 | 用途 |
| --- | --- | --- |
| `data/pca.csv` | 3643 | 现行快照与直筒子市镇街 |
| `data/pca_legacy.csv` | 15 | 历史、旧管理区和地点名称 |
| `data/pca_aliases.csv` | 10 | 有来源的显式地址别名 |

三张表与 Python fork 的 `882da68` 提交保持一致。上游快照为 `2025.251231.260403`（2026-04-03 采集），
官方补丁核验日期为 2026-09-08；包含和安县、和康县、岑岭县、重庆两江新区等已核验变更。
这些日期不代表全国所有最新变更都已收录。

东莞、中山、儋州、嘉峪关的镇街可放入 `district`；该字段不统一代表县级行政区。
国营蓝洋农场、松山湖等返回 `Place`。南朗镇返回 `Historical` 并建议南朗街道；
重庆江北区、渝北区需结合镇街确认现行归属，不提供统一替换建议。

兼容表不是全国历史区划全集。来源、哈希及更新方法见 [数据同步记录](https://github.com/laofahai/cpca-rs/blob/main/docs/data-update-2026.md)。

## 更新记录

| 版本 | 更新内容 |
| --- | --- |
| 0.3.0 | 同步现行、兼容和别名三张表；新增 `parse_with_status()` 与 `validate()`；保留原返回结构 |
| 0.3.0 | 修复中山市误识别、显式省份被覆盖、普通词误匹配、逆序父级及简称候选不确定性 |
| 0.3.0 | 新增真实地址边界和全表往返测试，更新 README，发布前检查测试与版本一致性 |
| 0.2.1 | CI 配置调整 |
| 0.2.0 | 调整同名地名解析策略 |

## 开发与验证

使用 Rust 稳定工具链：

```bash
cargo test --all-features
cargo fmt --all -- --check
cargo clippy --all-features --all-targets -- -D warnings
cargo doc --no-deps --all-features
cargo package
```

基于前缀树和归属索引实现，不依赖分词库。性能请使用仓库 benchmark 在目标机器上测量；本文不承诺固定延迟或跨语言倍数。

## 许可证

[MIT](LICENSE)
