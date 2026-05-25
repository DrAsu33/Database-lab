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

    // Friends module errors
    #[error("Target user does not exist")]
    UserDoesNotExist,

    #[error("Relation already exists or request is pending")]
    RelationAlreadyExists,

    #[error("Invalid relation state or request not found")]
    InvalidRelationState,

    #[error("The name of group already exists")]
    GroupNameAlreadyExists,

    #[error("The group does not exist")]
    GroupNotFound,

    #[error("The content mustn't be empty")]
    EmptyContent,

    #[error("The 150 alphabets limit was reached")]
    CharacterLimitExceeded,

    #[error("The moment or comment does not exist or you're unauthorized")]
    MomentOrCommentNotFoundOrUnauthorized
}