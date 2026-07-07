//! DSA 统一错误类型

use thiserror::Error;

/// DSA统一错误类型
#[derive(Error, Debug)]
pub enum DsaError {
    /// 配置错误
    #[error("配置错误: {0}")]
    Config(String),

    /// 行情数据获取失败
    #[error("行情数据获取失败: {0}")]
    StockData(String),

    /// LLM分析失败
    #[error("LLM 分析失败: {0}")]
    LlmAnalysis(String),

    /// 数据库操作失败
    #[error("数据库操作失败: {0}")]
    Database(String),

    /// 报告解析失败
    #[error("报告解析失败: {0}")]
    ReportParse(String),

    /// 回测计算失败
    #[error("回测计算失败: {0}")]
    Backtest(String),

    /// Agent对话失败
    #[error("Agent 对话失败: {0}")]
    Agent(String),

    /// 调度任务失败
    #[error("调度任务失败: {0}")]
    Scheduler(String),

    /// 参数校验失败
    #[error("参数校验失败: {0}")]
    Validation(String),

    /// API路由不存在
    #[error("API路由不存在: {0}")]
    ApiRouting(String),

    /// 内部错误
    #[error("{0}")]
    Internal(String),
}

impl From<std::io::Error> for DsaError {
    fn from(e: std::io::Error) -> Self {
        DsaError::Internal(e.to_string())
    }
}

impl From<serde_json::Error> for DsaError {
    fn from(e: serde_json::Error) -> Self {
        DsaError::ReportParse(e.to_string())
    }
}

/// DSA统一结果类型别名
pub type DsaResult<T> = Result<T, DsaError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let e = DsaError::StockData("timeout".to_string());
        assert_eq!(e.to_string(), "行情数据获取失败: timeout");

        let e = DsaError::Config("bad config".to_string());
        assert_eq!(e.to_string(), "配置错误: bad config");

        let e = DsaError::Validation("missing field".to_string());
        assert_eq!(e.to_string(), "参数校验失败: missing field");
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let dsa_err: DsaError = io_err.into();
        assert!(matches!(dsa_err, DsaError::Internal(_)));
    }

    #[test]
    fn test_result_type() {
        let ok: DsaResult<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: DsaResult<i32> = Err(DsaError::Database("conn failed".to_string()));
        assert!(err.is_err());
    }
}
