use sqlx::FromRow;
use sqlx::Type;
use std::fmt;
use serde::Serialize;

#[derive(Debug, PartialEq, Clone, Copy, Type, Serialize)]
#[repr(u8)] // 严格映射到底层 MySQL 的 TINYINT UNSIGNED
pub enum RelationStatus {
    Pending = 0,
    Accepted = 2,
    Stranger = 255, 
}

impl fmt::Display for RelationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            RelationStatus::Stranger => "Stranger",
            RelationStatus::Pending => "Request Sent",
            RelationStatus::Accepted => "Already Friends",
        };
        write!(f, "{}", text)
    }
}

// 用于返回搜索结果和好友列表
#[derive(Debug, FromRow, Serialize)]
pub struct FriendItem {
    pub id: u64,
    pub name: String,
    pub status: RelationStatus,
    pub group_name: Option<String>, //允许为 Option。如果是 NULL，说明该好友处于“默认/未分组”状态
}