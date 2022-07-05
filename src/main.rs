use config::Config;
use dotenvy::dotenv;
use wanikani::WaniKaniAPIClient;

mod config;
mod models;
mod wanikani;

#[tokio::main]
async fn main() -> reqwest::Result<()> {
    dotenv().ok();

    let config = Config::from_env();

    let client = WaniKaniAPIClient::new(&config.wanikani_api_key);

    let username = client.username().await?;
    println!("Welcome, {username}!");

    let radicals = client.radicals().await?;
    println!("There are {} radicals.", radicals.len());

    Ok(())
}
