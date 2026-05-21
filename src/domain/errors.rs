use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    // Business Rule Violations
    #[error("Invalid account or password")]
    InvalidCredentials,
    
    #[error("User already exists")]
    DuplicateUser,

    #[error("User not found")]
    UserNotFound,
    
    #[error("Invalid input: {0}")]
    _InvalidInput(String), // e.g.: 密码太弱、年龄不合法，附带具体原因   
    
    // 未来如果加入权限校验，只需在这里扩展
    #[error("User does not have permission")]
    _Unauthorized,

    // System failure! Print the error string
    #[error("Internal system failure: {0}")]
    SystemFailure(String),
}