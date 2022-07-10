use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub wanikani_api_key: String,
    pub session_key: String,
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    pub sentry_dsn: Option<String>,
}

fn default_bind_address() -> String {
    "127.0.0.1:3000".to_string()
}
