use std::env;

pub struct Config {
    pub wanikani_api_key: String,
}

impl Config {
    #[allow(clippy::wrong_self_convention)]
    pub fn from_env() -> Self {
        Self {
            wanikani_api_key: env::var("WANIKANI_API_KEY")
                .expect("WANIKANI_API_KEY environment variable is unset"),
        }
    }
}
