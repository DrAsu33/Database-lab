use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Invalid account or password")]
    InvalidCredentials,
    
    // 向外部隐藏数据库底层的具体 SQL 细节，仅在日志中保留
    #[error("Database constraint violation or connection error")]
    Database(#[from] sqlx::Error),

    #[error("Registration failed")]
    Registration(#[from] RegistrationError), 
    
    // 未来如果加入权限校验，只需在这里扩展
    #[error("User does not have permission")]
    _Unauthorized,
}

// 严谨的业务错误枚举
#[derive(Debug, Error)]
pub enum RegistrationError {
    #[error("Database error during Registration")]
    DatabaseError(sqlx::Error),
}