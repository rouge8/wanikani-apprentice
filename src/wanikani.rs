use crate::models::Radical;
use serde_json::Value;
use std::collections::HashMap;

pub struct WaniKaniAPIClient {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

enum SubjectType {
    Radical,
}

impl ToString for SubjectType {
    fn to_string(&self) -> String {
        match self {
            SubjectType::Radical => "radical".to_string(),
        }
    }
}

impl WaniKaniAPIClient {
    pub fn new(api_key: &str) -> Self {
        #[cfg(not(test))]
        let base_url = "https://api.wanikani.com/v2".to_string();

        #[cfg(test)]
        let base_url = mockito::server_url();

        Self {
            base_url,
            api_key: api_key.to_string(),
            client: reqwest::Client::new(),
        }
    }

    async fn request(
        &self,
        path: &str,
        params: Option<&HashMap<&str, &str>>,
    ) -> reqwest::Result<reqwest::Response> {
        let resp = self
            .client
            .get(format!("{}/{path}", self.base_url))
            .query(params.unwrap_or(&HashMap::new()))
            .header("Wanikani-Revision", "20170710")
            .bearer_auth(&self.api_key)
            .send()
            .await?;

        resp.error_for_status()
    }

    async fn subjects(&self, subject_type: SubjectType) -> reqwest::Result<Vec<Value>> {
        let mut next_url = Some("subjects".to_string());
        let mut results = Vec::new();

        while let Some(url) = &next_url {
            let subject_type = subject_type.to_string();
            let params = HashMap::from([("types", subject_type.as_str()), ("hidden", "false")]);
            let mut resp: Value = self.request(url, Some(&params)).await?.json().await?;

            next_url = resp["pages"]["next_url"].as_str().map(|s| s.to_string());
            if let Some(url) = next_url {
                next_url = Some(
                    url.splitn(2, &format!("{}/", self.base_url))
                        .collect::<Vec<&str>>()[1]
                        .to_string(),
                );
            }

            results.append(resp["data"].as_array_mut().unwrap());
        }

        Ok(results)
    }

    pub async fn radicals(&self) -> reqwest::Result<Vec<Radical>> {
        let mut results = Vec::new();

        for radical in self.subjects(SubjectType::Radical).await? {
            let character_svg_path = radical["data"]["character_images"]
                .as_array()
                .unwrap()
                .iter()
                .find_map(|image| {
                    if image["content_type"].as_str().unwrap() == "image/svg+xml"
                        && image["metadata"]["inline_styles"].as_bool().unwrap()
                    {
                        Some(
                            image["url"]
                                .as_str()
                                .unwrap()
                                .splitn(2, "https://files.wanikani.com/")
                                .collect::<Vec<&str>>()[1]
                                .to_string(),
                        )
                    } else {
                        None
                    }
                });

            results.push(Radical {
                id: radical["id"].as_u64().unwrap(),
                document_url: radical["data"]["document_url"]
                    .as_str()
                    .unwrap()
                    .to_string(),
                characters: radical["data"]["characters"]
                    .as_str()
                    .map(|s| s.to_string()),
                character_svg_path,
                meanings: radical["data"]["meanings"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|meaning| {
                        if meaning["accepted_answer"].as_bool().unwrap() {
                            Some(meaning["meaning"].as_str().unwrap().to_string())
                        } else {
                            None
                        }
                    })
                    .collect(),
            })
        }

        Ok(results)
    }

    pub async fn username(&self) -> reqwest::Result<String> {
        let resp: Value = self.request("user", None).await?.json().await?;

        Ok(resp["data"]["username"].as_str().unwrap().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{mock, Matcher};
    use pretty_assertions::assert_eq;
    use serde_json::json;

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

    #[tokio::test]
    async fn test_radicals() -> reqwest::Result<()> {
        let _m = mock("GET", "/subjects")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("types".into(), "radical".into()),
                Matcher::UrlEncoded("hidden".into(), "false".into()),
            ]))
            .with_status(200)
            .with_body(
                json!({
                    "data": [
                        {
                            "id": 1,
                            "object": "radical",
                            "data": {
                                "document_url": "https://www.wanikani.com/radicals/before",
                                "characters": "前",
                                "character_images": [],
                                "meanings": [
                                    {"meaning": "before", "primary": true, "accepted_answer": true},
                                ],
                            },
                        },
                        {
                            "id": 2,
                            "object": "radical",
                            "data": {
                                "document_url": "https://www.wanikani.com/radicals/belt",
                                "characters": "帯",
                                "character_images": [],
                                "meanings": [
                                    {"meaning": "lasso", "primary": false, "accepted_answer": false},
                                    {"meaning": "belt", "primary": true, "accepted_answer": true},
                                    {"meaning": "leather belt", "primary": false, "accepted_answer": true},
                                ],
                            },
                        },
                    ],
                    "pages": {
                        "next_url": None::<String>,
                    },
                })
                .to_string(),
            )
            .create();

        let client = WaniKaniAPIClient::new("fake-api-key");

        assert_eq!(
            client.radicals().await?,
            vec![
                Radical {
                    id: 1,
                    document_url: "https://www.wanikani.com/radicals/before".to_string(),
                    characters: Some("前".to_string()),
                    character_svg_path: None,
                    meanings: vec!["before".to_string()],
                },
                Radical {
                    id: 2,
                    document_url: "https://www.wanikani.com/radicals/belt".to_string(),
                    characters: Some("帯".to_string()),
                    character_svg_path: None,
                    meanings: vec!["belt".to_string(), "leather belt".to_string()],
                },
            ]
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_radicals_with_character_images() -> reqwest::Result<()> {
        let _m = mock("GET", "/subjects")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("types".into(), "radical".into()),
                Matcher::UrlEncoded("hidden".into(), "false".into()),
            ]))
            .with_status(200)
            .with_body(
                json!({
                    "data": [
                        {
                            "id": 1,
                            "object": "radical",
                            "data": {
                                "document_url": "https://www.wanikani.com/radicals/before",
                                "characters": None::<String>,
                                "character_images": [
                                    {
                                        "url": "https://files.wanikani.com/a.png",
                                        "content_type": "image/png",
                                    },
                                    {
                                        "url": "https://files.wanikani.com/the-good-path",
                                        "content_type": "image/svg+xml",
                                        "metadata": {
                                            "inline_styles": true,
                                        },
                                    },
                                    {
                                        "url": "https://files.wanikani.com/bad-svg",
                                        "content_type": "image/svg+xml",
                                        "metadata": {
                                            "inline_styles": false,
                                        },
                                    },
                                ],
                                "meanings": [
                                    {"meaning": "before", "primary": true, "accepted_answer": true},
                                ],
                            },
                        },
                    ],
                    "pages": {
                        "next_url": None::<String>,
                    },
                })
                .to_string(),
            )
            .create();

        let client = WaniKaniAPIClient::new("fake-api-key");

        assert_eq!(
            client.radicals().await?,
            vec![Radical {
                id: 1,
                document_url: "https://www.wanikani.com/radicals/before".to_string(),
                characters: None,
                character_svg_path: Some("the-good-path".to_string()),
                meanings: vec!["before".to_string()],
            },]
        );

        Ok(())
    }
}
