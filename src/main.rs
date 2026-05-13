use sqlx::mysql::{MySqlPoolOptions, MySqlPool};

// 严谨的业务错误枚举
#[derive(Debug)]
pub enum RegistrationError {
    DatabaseError(sqlx::Error),
}

// 用以下的函数来注册用户，只需要接受密码即可，id由数据库自增给出
async fn register_user(pool: &MySqlPool, raw_password: &str) -> Result<u64, RegistrationError> {
    let result = sqlx::query!(r#"INSERT INTO Users (password) VALUES (?)"#, raw_password)
    .execute(pool)
    .await
    .map_err(RegistrationError::DatabaseError)?;

    Ok(result.last_insert_id())
}

async fn get_pool() -> Result<MySqlPool, sqlx::Error> {
    // 确保读取.env文件中的URL
    // 环境变量中应包含 DATABASE_URL=mysql://user:password@localhost/db_name
    dotenvy::dotenv().ok();
    
    let database_url = std::env::var("DATABASE_URL")
    .expect("The environmental variant DATABASE_URL should be set.");

    // 1. 初始化并配置生产级连接池
    // 在调试阶段，最大最小链接数先设置为1，之后可以对参数进行修改。See above
    let pool = MySqlPoolOptions::new()
        .max_connections(MAX_CONNECTION)
        .min_connections(MIN_CONNECTION)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect(&database_url)
        .await?;

    Ok(pool)
}

// 在调试阶段，最大最小链接数先设置为1，之后可以对参数进行修改。
const MAX_CONNECTION : u32 = 1;
const MIN_CONNECTION : u32 = 1;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {

    let pool = get_pool().await?;

    let new_user_id= register_user(&pool, "123456")
    .await
    .expect("User Registration FAILED due to some anomalies.");

    println!("Registration success. Your assigned ID is {}", new_user_id);
    Ok(())
}