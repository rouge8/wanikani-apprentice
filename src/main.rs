use config::Config;
use dotenvy::dotenv;
use tracing_subscriber::FmtSubscriber;
use wanikani::WaniKaniAPIClient;

mod config;
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

    let radicals = client.radicals().await?;
    println!("There are {} radicals.", radicals.len());

    let kanji = client.kanji().await?;
    println!("There are {} kanji.", kanji.len());

    let vocabulary = client.vocabulary().await?;
    println!("There are {} vocabulary.", vocabulary.len());

    Ok(())
}
