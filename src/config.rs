use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub wanikani_api_key: String,
    pub session_key: String,
}
