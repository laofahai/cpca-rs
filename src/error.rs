//! 错误类型定义

use thiserror::Error;

/// 地址解析错误
#[derive(Debug, Error)]
pub enum ParseError {
    /// 数据加载失败
    #[error("Failed to load region data: {0}")]
    DataLoadError(String),

    /// 无效的地址格式
    #[error("Invalid address format: {0}")]
    InvalidFormat(String),

    /// 未找到匹配的地区
    #[error("No matching region found for: {0}")]
    NotFound(String),
}
