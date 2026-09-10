# Changelog

## 0.3.0 (2026-09-10)

- 同步 2026 区划快照、15 条历史兼容与 10 条显式别名，记录数据来源及哈希。
- 新增 `parse_with_status()`、`validate()` 和状态类型，保留 `ParsedAddress` 及旧 `parse()` 返回结构。
- 修复完整市县名称优先级、跨省覆盖、普通词误匹配和逆序父级边界；不任意选择重名区候选。
- 增加全表往返、真实地址回归与校验测试，整理 README，发布前验证测试及标签版本。

## [0.2.1](https://github.com/laofahai/cpca-rs/compare/v0.2.0...v0.2.1) (2026-01-12)


### Bug Fixes

* 移除 CI 中的 target 缓存 ([b17c119](https://github.com/laofahai/cpca-rs/commit/b17c11909a923c175d3b7791598f93907536126f))

## [0.2.0](https://github.com/laofahai/cpca-rs/compare/v0.1.0...v0.2.0) (2026-01-12)


### Features

* 优化同名地名解析策略 ([0dcd10a](https://github.com/laofahai/cpca-rs/commit/0dcd10af9c9fa13d395017e189699e7aa8d16095))

## 0.1.0 (2026-01-12)


### Features

* initial commit - Chinese Province City Area Parser ([48d724a](https://github.com/laofahai/cpca-rs/commit/48d724a7f128f234542fdd4b9b87c2d1580f04d5))


### Bug Fixes

* correct rust-toolchain action name ([731e0e7](https://github.com/laofahai/cpca-rs/commit/731e0e713d56b0d7e1636527ad48fb24e7d4c6e7))
* resolve clippy warnings and format code ([6bdec04](https://github.com/laofahai/cpca-rs/commit/6bdec04d82ef2641f481060e8d106ff6a6221f18))
