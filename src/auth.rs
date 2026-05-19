use sqlx::mysql::MySqlPool;
use crate::app::App;


// 严谨的业务错误枚举
#[derive(Debug)]
pub enum RegistrationError {
    DatabaseError(sqlx::Error),
}

// Following func is supposed to interact with the database, not the user.
impl App {
    
    pub async fn login_with_data(&mut self, account: i32, password: &str) {

    }

    // 用以下的函数来注册用户，只需要接受密码即可，id由数据库自增给出
    pub async fn register_with_password(&self, sql_pool: &MySqlPool, raw_password: &str) -> Result<u64, RegistrationError> {
        let result = sqlx::query!(r#"INSERT INTO Users (password) VALUES (?)"#, raw_password)
        .execute(sql_pool)
        .await
        .map_err(RegistrationError::DatabaseError)?;

        Ok(result.last_insert_id())
    }
}