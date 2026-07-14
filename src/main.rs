
mod service;
mod domain;
mod infrastructure;
mod web;
use sqlx::mysql::{MySqlPoolOptions, MySqlPool};
use std::sync::Arc;
use std::net::SocketAddr;

use crate::infrastructure::{MySqlRelationRepository, MySqlUserRepository, MySqlMomentRepository};
use crate::web::AppState;

// 在调试阶段，最大最小链接数先设置为1，之后可以对参数进行修改。
const MAX_CONNECTION : u32 = 10;
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

async fn compose_app_state() -> Result<Arc<AppState>, Box<dyn std::error::Error>> {
    let pool = get_pool().await?;
    let user_repository = Arc::new(MySqlUserRepository::new(pool.clone()));
    let relation_repository = Arc::new(MySqlRelationRepository::new(pool.clone()));
    let moment_repository = Arc::new(MySqlMomentRepository::new(pool.clone()));

    let user_service = Arc::new(service::UserService::new(user_repository));
    let relation_service = Arc::new(service::RelationService::new(relation_repository));
    let moment_service = Arc::new(service::MomentService::new(moment_repository));

    // 加载 templates/ 目录下所有 .html 模板
    let tera = tera::Tera::new("templates/**/*.html")
        .map_err(|e| format!("无法加载模板: {}", e))?;

    println!("[OK] 已加载 {} 个模板", tera.get_template_names().count());

    Ok(Arc::new(AppState {
        tera,
        user_service,
        relation_service,
        moment_service,
    }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = compose_app_state().await?;

    let app = web::create_router(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("====================================");
    println!("  朋友圈系统 (Web 版)");
    println!("  访问地址 → http://{}", addr);
    println!("====================================");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}