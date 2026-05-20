use sqlx::mysql::MySqlPool;
use crate::errors::RegistrationError;

// Following func is supposed to interact with the database, not the user.
pub async fn login_with_data(sql_pool: &MySqlPool, account: u64, password: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query_scalar!(r#"SELECT 1 FROM Users u WHERE id = (?) AND password = (?)"#, account, password)
    .fetch_optional(sql_pool)
    .await?;
    
    Ok(result.is_some())
}

// 用以下的函数来注册用户，只需要接受密码即可，id由数据库自增给出
pub async fn register_with_password(sql_pool: &MySqlPool, raw_password: &str) -> Result<u64, RegistrationError> {
    let result = sqlx::query!(r#"INSERT INTO Users (password) VALUES (?)"#, raw_password)
    .execute(sql_pool)
    .await
    .map_err(RegistrationError::DatabaseError)?;

    Ok(result.last_insert_id())
}