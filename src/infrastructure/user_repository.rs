use sqlx::MySqlPool;
use async_trait::async_trait;
use crate::service::UserRepository;
use crate::domain::{DomainError, UserProfile, Role};

pub struct MySqlUserRepository {
    pool: MySqlPool,
}

// The following fns communicate with the database.
// Note: the executor shall be a reference because the pool shall be reused
impl MySqlUserRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for MySqlUserRepository {
    async fn login_with_data(&self, account: u64, password: &str) -> Result<Role, DomainError> {
        let result = sqlx::query_scalar!(r#"SELECT role AS "role: Role" FROM Users u WHERE id = (?) AND password = (?)"#, account, password)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            DomainError::SystemFailure(e.to_string())
        })?;
        
        match result {
            Some(role_value) => Ok(role_value),
            None => Err(DomainError::InvalidCredentials),
        }
    }

    async fn register_with_password(&self, raw_password: &str) -> Result<u64, DomainError> {
        let result = sqlx::query!(r#"INSERT INTO Users (password) VALUES (?)"#, raw_password)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            match e {
                // catches "unique_violation", and translates it into DuplicateUser
                sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                    DomainError::DuplicateUser
                }
                // Other errors are converted to String and propagated
                _ => DomainError::SystemFailure(e.to_string()),
            }
        })?;

        Ok(result.last_insert_id())
    }

    async fn fetch_profile(&self, uid: u64) -> Result<UserProfile, DomainError> {
        let user = sqlx::query_as!(
            UserProfile,
            r#"SELECT id, name, gender, birth_date FROM Users WHERE id = ? AND role = ?"#,
            uid, Role::User
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            match e {
                // catches "row_not_found", and translates it into UserNotFound
                sqlx::Error::RowNotFound => DomainError::UserNotFound,
                _ => DomainError::SystemFailure(e.to_string()),
            }
        })?;

        // The struct UserProfile has derived "FromRow"
        Ok(user)
    }

    async fn save_profile(&self, profile: &UserProfile) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE Users 
            SET name = ?, gender = ?, birth_date = ?
            WHERE id = ?
            "#,
            profile.name,
            profile.gender,
            profile.birth_date,
            profile.id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            DomainError::SystemFailure(e.to_string())
        })?;

        Ok(())
    }

    async fn force_cancel_user(&self, uid: u64) -> Result<(), DomainError> {
        let result = sqlx::query!(r#"DELETE FROM Users WHERE id = ? AND role = ?"#, uid, Role::User)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::SystemFailure(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::UserNotFound);
        }

        Ok(())   
    }
}