mod cli;
mod service;
mod domain;
mod infrastructure;
use sqlx::mysql::{MySqlPoolOptions, MySqlPool};
use std::sync::Arc;

use crate::infrastructure::{MySqlRelationRepository, MySqlUserRepository, MySqlMomentRepository};

// 在调试阶段，最大最小链接数先设置为1，之后可以对参数进行修改。
const MAX_CONNECTION : u32 = 1;
const MIN_CONNECTION : u32 = 1;

// fn to get the connection pool with MySQL
async fn get_pool() -> Result<MySqlPool, sqlx::Error> {
    // Makes sure the URL in .env is read.
    // in .env there should be:
    // DATABASE_URL=mysql://lab_user:lab_password@localhost:3306/lab_DB
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
    .expect("The environmental variant DATABASE_URL should be set.");

    // Initialize the connection pool.
    // The parameters of MAX(MIN)_CONNECTION are in App.rs
    MySqlPoolOptions::new()
        .max_connections(MAX_CONNECTION)
        .min_connections(MIN_CONNECTION)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect(&database_url)
        .await
}

async fn compose_cli() -> Result<cli::CliApplication, sqlx::Error> {
    let pool = get_pool().await?;
    let user_repository = MySqlUserRepository::new(pool.clone());
    let relation_repository = MySqlRelationRepository::new(pool.clone());
    let moment_repository = MySqlMomentRepository::new(pool.clone());

    Ok(
        cli::CliApplication::new(
            service::UserService::new(Arc::new(user_repository)),
            service::RelationService::new(Arc::new(relation_repository)),
            service::MomentService::new(Arc::new(moment_repository))
        )
    )
}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let mut client_app = compose_cli().await?;
    client_app.run().await?;

    println!("Successfully quitted!");
    Ok(())
}