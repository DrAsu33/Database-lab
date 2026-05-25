use sqlx::MySqlPool;
use crate::{domain::{DomainError, MomentItem, CommentItem}, service::MomentRepository};
use async_trait::async_trait;

pub const MOMENT_LIMIT: usize = 150;
pub const COMMENT_LIMIT: usize = 50;

pub struct MySqlMomentRepository {
    pool: MySqlPool,
}

impl MySqlMomentRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

// Note: in infra layer the content of moments/comments MUST HAVE BEEN TRIMMED already!
#[async_trait]
impl MomentRepository for MySqlMomentRepository {
    async fn fetch_friends_moments(&self, user_id: u64) -> Result<Vec<MomentItem>, DomainError> {
        let res = sqlx::query_as!(
            MomentItem,
            r#"
            SELECT 
                m.moment_id AS "moment_id!",
                m.author_id AS "author_id!",
                u1.name AS author_name,
                m.content AS "content!",
                m.created_at AS "created_at!",
                m.last_modified_time AS "last_modified_time!",
                COALESCE(
                    (
                        SELECT JSON_ARRAYAGG(
                            JSON_OBJECT(
                                'comment_id', c.comment_id,
                                'moment_id', c.moment_id,
                                'commenter_id', c.commenter_id,
                                'commenter_name', u2.name,
                                'comment', c.comment,
                                'created_at', DATE_FORMAT(c.created_at, '%Y-%m-%dT%H:%i:%sZ')
                            )
                        )
                        FROM Comments c
                        JOIN Users u2 ON c.commenter_id = u2.id
                        WHERE c.moment_id = m.moment_id
                    ), 
                    JSON_ARRAY()
                ) AS "comments!: sqlx::types::Json<Vec<CommentItem>>"
            FROM Moments m
            JOIN Users u1 ON m.author_id = u1.id
            WHERE m.author_id = ? OR m.author_id IN (
                SELECT friend_id FROM Relations WHERE user_id = ? AND status = 2
            )
            ORDER BY m.created_at DESC
            LIMIT 50;
            "#,
            user_id, user_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::SystemFailure(e.to_string()))?;

        Ok(res)
    }

    async fn post_moment(&self, uid: u64, content: &str) -> Result<u64, DomainError> {
        let result = sqlx::query!(
            r#"INSERT INTO Moments (author_id, content) VALUES (?, ?)"#,
            uid, content
        )
        .execute(&self.pool)
        .await;

        match result {
            Ok(res) => Ok(res.last_insert_id()),
            Err(sqlx::Error::Database(db_err)) => {
                // 23000 是违反约束的通用 SQLSTATE，1452 是外键失败
                if db_err.code().as_deref() == Some("23000") && db_err.message().contains("foreign key") {
                    Err(DomainError::UserNotFound) // 发朋友圈的人不存在
                } else {
                    Err(DomainError::SystemFailure(db_err.to_string()))
                }
            }
            Err(e) => Err(DomainError::SystemFailure(e.to_string())),
        }
    }

    async fn update_moment(&self, uid: u64, moment_id: u64, new_content: &str) -> Result<(), DomainError> {
        let result = sqlx::query!(
            r#"UPDATE Moments SET content = ? WHERE moment_id = ? AND author_id = ?"#,
            new_content, moment_id, uid
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::SystemFailure(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::MomentOrCommentNotFoundOrUnauthorized); 
        }

        Ok(())
    }

    async fn delete_moment(&self, uid: u64, moment_id: u64) -> Result<(), DomainError> {
        let result = sqlx::query!(
            r#"DELETE FROM Moments WHERE moment_id = ? AND author_id = ?"#,
            moment_id, uid
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::SystemFailure(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::MomentOrCommentNotFoundOrUnauthorized);
        }

        Ok(())
    }

    async fn post_comment(&self, uid: u64, moment_id: u64, content: &str) -> Result<u64, DomainError> {
        let result = sqlx::query!(
            r#"INSERT INTO Comments (moment_id, commenter_id, comment) VALUES (?, ?, ?)"#,
            moment_id, uid, content
        )
        .execute(&self.pool)
        .await;
  
        match result {
            Ok(res) => Ok(res.last_insert_id()),
            Err(sqlx::Error::Database(db_err)) => {
                // 23000 是违反约束的通用 SQLSTATE，1452 是外键失败
                if db_err.code().as_deref() == Some("23000") && db_err.message().contains("foreign key") {
                    Err(DomainError::UserNotFound) // 发朋友圈的人不存在
                } else {
                    Err(DomainError::SystemFailure(db_err.to_string()))
                }
            }
            Err(e) => Err(DomainError::SystemFailure(e.to_string())),
        }
    }

    async fn delete_comment(&self, uid: u64, comment_id: u64) -> Result<(), DomainError> {
        let result = sqlx::query!(
            r#"
            DELETE c FROM Comments c
            JOIN Moments m ON c.moment_id = m.moment_id
            WHERE comment_id = ? AND (commenter_id = ? OR m.author_id = ?)
            "#,
            comment_id, uid, uid
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::SystemFailure(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::MomentOrCommentNotFoundOrUnauthorized);
        }

        Ok(())
    }

}