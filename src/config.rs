use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub wanikani_api_key: String,
    #[serde(default = "default_wanikani_api_url")]
    pub wanikani_api_url: String,
    #[serde(default = "default_wanikani_files_server_url")]
    pub wanikani_files_server_url: String,
    pub session_key: String,
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    pub sentry_dsn: Option<String>,
    pub trusted_hosts: Vec<String>,
}

fn default_wanikani_api_url() -> String {
    "https://api.wanikani.com/v2".to_string()
}

fn default_wanikani_files_server_url() -> String {
    "https://files.wanikani.com".to_string()
}

fn default_bind_address() -> String {
    "127.0.0.1:3000".to_string()
}
