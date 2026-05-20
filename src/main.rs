mod auth;
mod service;
mod errors;
mod cli;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let mut client_app = cli::CliApplication::new().await?;
    client_app.run().await?;

    println!("Successfully quitted!");
    Ok(())
}