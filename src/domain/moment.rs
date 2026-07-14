use chrono::{DateTime, Utc};
use sqlx::FromRow;
use serde::{Deserialize, Serialize};

// 评论作为子实体，必须实现 Deserialize，以便 sqlx 将 MySQL 返回的 JSON 数组直接反序列化
#[derive(Debug, FromRow, Deserialize, Serialize)]
pub struct MomentItem {
    pub moment_id: u64,
    pub author_id: u64,
    pub author_name: Option<String>,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub last_modified_time: DateTime<Utc>,
    pub comments: Vec<CommentItem>,
}

#[derive(Debug, FromRow, Deserialize, Serialize)]
pub struct CommentItem {
    pub comment_id: u64,
    pub moment_id: u64,
    pub commenter_id: u64,
    pub commenter_name: Option<String>,
    pub comment: String,
    pub created_at: DateTime<Utc>
}