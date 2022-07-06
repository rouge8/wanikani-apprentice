use config::Config;
use db::Database;
use dotenvy::dotenv;
use tracing_subscriber::FmtSubscriber;
use wanikani::WaniKaniAPIClient;

mod config;
mod db;
mod models;
mod wanikani;

#[tokio::main]
async fn main() -> reqwest::Result<()> {
    dotenv().ok();

    let subscriber = FmtSubscriber::builder().finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let config = Config::from_env();

    let client = WaniKaniAPIClient::new(&config.wanikani_api_key);

    let username = client.username().await?;
    println!("Welcome, {username}!");

    let mut db = Database::new();
    db.populate(&client).await?;

    let assignments = client.assignments(&db).await?;
    println!("You have {} assignments.", assignments.len());

    Ok(())
}
