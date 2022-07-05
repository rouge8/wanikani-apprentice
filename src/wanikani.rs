use serde_json::Value;
use std::collections::HashMap;

pub struct WaniKaniAPIClient {
    api_key: String,
    client: reqwest::Client,
}

impl WaniKaniAPIClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_owned(),
            client: reqwest::Client::new(),
        }
    }

    async fn request(
        &self,
        path: &str,
        params: Option<&HashMap<&str, &str>>,
    ) -> reqwest::Result<reqwest::Response> {
        #[cfg(not(test))]
        let base_url = "https://api.wanikani.com/v2";

        #[cfg(test)]
        let base_url = &mockito::server_url();

        let resp = self
            .client
            .get(format!("{base_url}/{path}"))
            .query(params.unwrap_or(&HashMap::new()))
            .header("Wanikani-Revision", "20170710")
            .bearer_auth(&self.api_key)
            .send()
            .await?;

        resp.error_for_status()
    }

    pub async fn username(&self) -> reqwest::Result<String> {
        let resp: Value = self.request("user", None).await?.json().await?;

        match &resp["data"]["username"] {
            Value::String(username) => Ok(username.to_owned()),
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::mock;
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn test_username() -> reqwest::Result<()> {
        let _m = mock("GET", "/user")
            .with_status(200)
            .with_body(r#"{"data": {"username": "test-user"}}"#)
            .create();

        let client = WaniKaniAPIClient::new("fake-api-key");

        assert_eq!(client.username().await?, "test-user");

        Ok(())
    }
}
