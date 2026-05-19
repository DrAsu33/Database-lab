mod app;
mod auth;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let mut app = app::App::new().await?;
    app.run().await;

    println!("Successfully quitted!");
    Ok(())
}