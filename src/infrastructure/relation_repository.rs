use sqlx::MySqlPool;
use async_trait::async_trait;
use crate::service::RelationRepository;
use crate::domain::{DomainError, relation::FriendItem, RelationStatus};

pub struct MySqlRelationRepository {
    pool: MySqlPool,
}

impl MySqlRelationRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RelationRepository for MySqlRelationRepository {
    async fn search_users(&self, current_user_id: u64, query: &str) -> Result<Vec<FriendItem>, DomainError> {
        // 使用前缀匹配走索引，同时左连接查询当前用户与他们的关系状态
        let like_query = format!("{}%", query);
        let users = sqlx::query_as!(
            FriendItem,
            r#"
            SELECT u.id,
            COALESCE(u.name, '') as "name!: String",
            COALESCE(r.status, 255) as "status!: RelationStatus",
            fg.name as "group_name?: String"
            FROM Users u
            LEFT JOIN Relations r ON u.id = r.friend_id AND r.user_id = ?
            LEFT JOIN FriendGroups fg ON r.group_id = fg.group_id
            WHERE u.id != ? AND u.name LIKE ?
            LIMIT 50
            "#,
            current_user_id,current_user_id , like_query
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::SystemFailure(e.to_string()))?;

        Ok(users)
    }

    async fn send_request(&self, user_id: u64, target_id: u64) -> Result<(), DomainError> {
        if user_id == target_id {
            return Err(DomainError::SystemFailure("Cannot add yourself".into()));
        }

        let result = sqlx::query!(
            r#"INSERT INTO Relations (user_id, friend_id, status) VALUES (?, ?, 0)"#,
            user_id, target_id
        )
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(db_err)) => {
                match db_err.code().as_deref() {
                    Some("23000") if db_err.message().contains("foreign key") => Err(DomainError::UserDoesNotExist),
                    Some("23000") if db_err.message().contains("Duplicate entry") => Err(DomainError::RelationAlreadyExists),
                    _ => Err(DomainError::SystemFailure(db_err.to_string())),
                }
            }
            Err(e) => Err(DomainError::SystemFailure(e.to_string())),
        }
    }

    // 注意，接下来两个函数必须使用transaction机制保证完整性。
    async fn accept_request(&self, user_id: u64, applicant_id: u64) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(|e| DomainError::SystemFailure(e.to_string()))?;

        // 1. 将对方单向的申请状态 0 改为 2
        let update_res = sqlx::query!(
            r#"UPDATE Relations SET status = 2 WHERE user_id = ? AND friend_id = ? AND status = 0"#,
            applicant_id, user_id
        )
        .execute(&mut *tx).await.map_err(|e| DomainError::SystemFailure(e.to_string()))?;

        if update_res.rows_affected() == 0 {
            return Err(DomainError::InvalidRelationState);
        }

        // 2. 插入我对对方的记录，状态 2
        sqlx::query!(
            r#"INSERT INTO Relations (user_id, friend_id, status) VALUES (?, ?, 2)"#,
            user_id, applicant_id
        )
        .execute(&mut *tx).await.map_err(|e| DomainError::SystemFailure(e.to_string()))?;

        tx.commit().await.map_err(|e| DomainError::SystemFailure(e.to_string()))?;
        Ok(())
    }

    // Note: the following fn also deletes the pending request
    async fn remove_friend(&self, user_id: u64, friend_id: u64) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(|e| DomainError::SystemFailure(e.to_string()))?;

        let res1 = sqlx::query!(r#"DELETE FROM Relations WHERE user_id = ? AND friend_id = ?"#, user_id, friend_id)
            .execute(&mut *tx).await.map_err(|e| DomainError::SystemFailure(e.to_string()))?;

        let res2 = sqlx::query!(r#"DELETE FROM Relations WHERE user_id = ? AND friend_id = ?"#, friend_id, user_id)
            .execute(&mut *tx).await.map_err(|e| DomainError::SystemFailure(e.to_string()))?;

        if res1.rows_affected() + res2.rows_affected() == 0 {
            // 这里不需要手动调用 tx.rollback()。
            // 因为直接 return Err，tx 变量会离开作用域 (Drop)，
            // sqlx 的底层机制会自动向数据库发送 ROLLBACK。
            return Err(DomainError::InvalidRelationState); // 强烈建议你在 DomainError 中添加这个变体
        }
        tx.commit().await.map_err(|e| DomainError::SystemFailure(e.to_string()))?;
        Ok(())
    }

    async fn list_relations(&self, user_id: u64, status: RelationStatus) -> Result<Vec<FriendItem>, DomainError> {
        let friends = sqlx::query_as!(
            FriendItem,
            r#"
            SELECT u.id, COALESCE(u.name, '') AS "name!: String",
            r.status AS "status!: RelationStatus",
            fg.name AS group_name
            FROM Relations r
            JOIN Users u ON r.friend_id = u.id
            LEFT JOIN FriendGroups fg ON r.group_id = fg.group_id
            WHERE r.user_id = ? AND r.status = ?
            "#,
            user_id, status
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::SystemFailure(e.to_string()))?;

        Ok(friends)
    }

    async fn list_pending_requests(&self, my_user_id: u64) -> Result<Vec<FriendItem>, DomainError> {
        let requests = sqlx::query_as!(
            FriendItem,
            r#"
            SELECT u.id,
            COALESCE(u.name, '') as "name!: String",
            r.status as "status!: RelationStatus",
            NULL as "group_name?: String"
            FROM Relations r
            JOIN Users u ON r.user_id = u.id
            WHERE r.friend_id = ? AND r.status = ?
            "#,
            my_user_id, RelationStatus::Pending
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::SystemFailure(e.to_string()))?;

        Ok(requests)
    }

    async fn create_group(&self, user_id: u64, group_name: &str) -> Result<u64, DomainError> {
        let result = sqlx::query!(
            r#"INSERT INTO FriendGroups (user_id, name) VALUES (?, ?)"#,
            user_id, group_name
        )
        .execute(&self.pool)
        .await;

        match result {
            Ok(res) => Ok(res.last_insert_id()),
            Err(sqlx::Error::Database(db_err)) => {
                if db_err.code().as_deref() == Some("23000") && db_err.message().contains("Duplicate entry") {
                    Err(DomainError::GroupNameAlreadyExists) 
                } else {
                    Err(DomainError::SystemFailure(db_err.to_string()))
                }
            }
            Err(e) => Err(DomainError::SystemFailure(e.to_string())),
        }
    }

    async fn delete_group(&self, user_id: u64, group_id: u64) -> Result<(), DomainError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM FriendGroups
            WHERE group_id = ? AND user_id = ?
            "#,
        group_id, user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::SystemFailure(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::GroupNotFound);
        }
        Ok(())
    }

    // group_id: Option<u64>：Some(id) => move into the group，None => move out of the group(SET AS NULL)
    async fn set_friend_group(&self, user_id: u64, friend_id: u64, group_id: Option<u64>) -> Result<(), DomainError> {
        // Should limit: status = 2，since you cannot group a non-friend user!
        let update_res = sqlx::query!(
            r#"
            UPDATE Relations 
            SET group_id = ? 
            WHERE user_id = ? AND friend_id = ? AND status = 2
            "#,
            group_id, user_id, friend_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::SystemFailure(e.to_string()))?;

        if update_res.rows_affected() == 0 {
            // 如果没有更新，说明这两人根本不是正式好友，或者输入的好友ID不对
            return Err(DomainError::InvalidRelationState);
        }

        Ok(())
    }

    async fn get_group_id_by_name(&self, user_id: u64, group_name: &str) -> Result<Option<u64>, DomainError> {
        let group_id = sqlx::query_scalar!(
            r#"
            SELECT group_id
            FROM FriendGroups
            WHERE user_id = ? AND name = ?
            "#,
            user_id, group_name
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::SystemFailure(e.to_string()))?;

        Ok(group_id)
    }
}