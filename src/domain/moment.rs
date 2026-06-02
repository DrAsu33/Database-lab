use chrono::{DateTime, Utc};
use sqlx::FromRow;
use serde::{Deserialize, Serialize};
use chrono::Local;

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

impl MomentItem {
    fn is_edited(&self) -> bool {
        self.last_modified_time > self.created_at
    }

    pub fn display(&self) {
        let edited_tag = self.is_edited();
        let time_str = if edited_tag {
            self.last_modified_time.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string()
        }
        else {
            self.created_at.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string()
        };

        if self.is_edited() {
            println!("\n[ID: {}] By {} Last edited at: {}", self.moment_id, self.author_name.as_deref().unwrap_or_default(), time_str);
        }
        else {
            println!("\n[ID: {}] By {} Posted at: {}", self.moment_id, self.author_name.as_deref().unwrap_or_default(), time_str);
        }
        println!("  {}", self.content);
        
        let comments = &self.comments; 
        if !comments.is_empty() {
            println!("  --- Comments ---");
            for c in comments {
                let c_time = c.created_at.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string();
                println!("    -> [comment ID: {}] {}: {} (commented at: {})", 
                    c.comment_id, c.commenter_name.as_deref().unwrap_or(""), c.comment, c_time);
            }
        }
        println!("----------------------------------------");
    }
}