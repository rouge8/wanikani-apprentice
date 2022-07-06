use crate::models::{Kanji, Radical, Vocabulary};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Instant;
use tracing::info;

pub struct WaniKaniAPIClient {
    pub base_url: String,
    api_key: String,
    client: reqwest::Client,
}

enum SubjectType {
    Radical,
    Kanji,
    Vocabulary,
}

impl ToString for SubjectType {
    fn to_string(&self) -> String {
        match self {
            SubjectType::Radical => "radical".to_string(),
            SubjectType::Kanji => "kanji".to_string(),
            SubjectType::Vocabulary => "vocabulary".to_string(),
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
        info!(path, params = ?params, "requesting");
        let start = Instant::now();
        let resp = self
            .client
            .get(format!("{}/{path}", self.base_url))
            .query(params.unwrap_or(&HashMap::new()))
            .header("Wanikani-Revision", "20170710")
            .bearer_auth(&self.api_key)
            .send()
            .await?;
        let end = start.elapsed();
        info!(
            path,
            params = ?params,
            status_code = resp.status().as_u16(),
            duration = end.as_secs_f32(),
            "requested",
        );

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
            });
        }

        Ok(results)
    }

    pub async fn kanji(&self) -> reqwest::Result<Vec<Kanji>> {
        let mut results = Vec::new();

        for kanji in self.subjects(SubjectType::Kanji).await? {
            results.push(Kanji {
                id: kanji["id"].as_u64().unwrap(),
                document_url: kanji["data"]["document_url"].as_str().unwrap().to_string(),
                characters: kanji["data"]["characters"].as_str().unwrap().to_string(),
                meanings: kanji["data"]["meanings"]
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
                readings: kanji["data"]["readings"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|reading| {
                        if reading["accepted_answer"].as_bool().unwrap() {
                            Some(reading["reading"].as_str().unwrap().to_string())
                        } else {
                            None
                        }
                    })
                    .collect(),
            });
        }

        Ok(results)
    }

    pub async fn vocabulary(&self) -> reqwest::Result<Vec<Vocabulary>> {
        let mut results = Vec::new();

        for vocab in self.subjects(SubjectType::Vocabulary).await? {
            results.push(Vocabulary {
                id: vocab["id"].as_u64().unwrap(),
                document_url: vocab["data"]["document_url"].as_str().unwrap().to_string(),
                characters: vocab["data"]["characters"].as_str().unwrap().to_string(),
                meanings: vocab["data"]["meanings"]
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
                readings: vocab["data"]["readings"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|reading| {
                        if reading["accepted_answer"].as_bool().unwrap() {
                            Some(reading["reading"].as_str().unwrap().to_string())
                        } else {
                            None
                        }
                    })
                    .collect(),
            });
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

    #[tokio::test]
    async fn test_kanji() -> reqwest::Result<()> {
        let client = WaniKaniAPIClient::new("fake-api-key");

        let _page1 = mock("GET", "/subjects")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("types".into(), "kanji".into()),
                Matcher::UrlEncoded("hidden".into(), "false".into()),
            ]))
            .with_status(200)
            .with_body(
                json!({
                    "data": [
                        {
                            "id": 1,
                            "object": "kanji",
                            "data": {
                                "document_url": "https://www.wanikani.com/kanji/a",
                                "characters": "a",
                                "meanings": [
                                    {"meaning": "a1", "primary": true, "accepted_answer": true},
                                    {"meaning": "a2", "primary": false, "accepted_answer": false},
                                    {"meaning": "a3", "primary": false, "accepted_answer": true},
                                ],
                                "readings": [
                                    {
                                        "type": "type1",
                                        "primary": true,
                                        "reading": "a",
                                        "accepted_answer": true,
                                    },
                                    {
                                        "type": "type1",
                                        "primary": false,
                                        "reading": "b",
                                        "accepted_answer": true,
                                    },
                                    {
                                        "type": "type2",
                                        "primary": false,
                                        "reading": "c",
                                        "accepted_answer": true,
                                    },
                                    {
                                        "type": "type2",
                                        "primary": false,
                                        "reading": "d",
                                        "accepted_answer": false,
                                    },
                                ],
                            },
                        },
                    ],
                    "pages": {
                        "next_url": format!("{}/subjects?types=kanji&hidden=false&page_after_id=1", client.base_url),
                    },
                })
                .to_string(),
            )
            .create();
        let _page2 = mock("GET", "/subjects")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("types".into(), "kanji".into()),
                Matcher::UrlEncoded("hidden".into(), "false".into()),
                Matcher::UrlEncoded("page_after_id".into(), "1".into()),
            ]))
            .with_status(200)
            .with_body(
                json!({
                    "data": [
                        {
                            "id": 2,
                            "object": "kanji",
                            "data": {
                                "document_url": "https://www.wanikani.com/kanji/b",
                                "characters": "b",
                                "meanings": [
                                    {"meaning": "b", "primary": true, "accepted_answer": true},
                                ],
                                "readings": [
                                    {
                                        "type": "type1",
                                        "primary": true,
                                        "reading": "b",
                                        "accepted_answer": true,
                                    },
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

        assert_eq!(
            client.kanji().await?,
            vec![
                Kanji {
                    id: 1,
                    document_url: "https://www.wanikani.com/kanji/a".to_string(),
                    characters: "a".to_string(),
                    meanings: vec!["a1".to_string(), "a3".to_string()],
                    readings: vec!["a".to_string(), "b".to_string(), "c".to_string()],
                },
                Kanji {
                    id: 2,
                    document_url: "https://www.wanikani.com/kanji/b".to_string(),
                    characters: "b".to_string(),
                    meanings: vec!["b".to_string()],
                    readings: vec!["b".to_string()],
                },
            ]
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_vocabulary() -> reqwest::Result<()> {
        let client = WaniKaniAPIClient::new("fake-api-key");

        let _page1 = mock("GET", "/subjects")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("types".into(), "vocabulary".into()),
                Matcher::UrlEncoded("hidden".into(), "false".into()),
            ]))
            .with_status(200)
            .with_body(
                json!({
                    "data": [
                        {
                            "id": 1,
                            "object": "vocabulary",
                            "data": {
                                "document_url": "https://www.wanikani.com/vocabulary/a",
                                "characters": "a",
                                "meanings": [
                                    {"meaning": "a1", "primary": true, "accepted_answer": true},
                                    {"meaning": "a2", "primary": false, "accepted_answer": false},
                                    {"meaning": "a3", "primary": false, "accepted_answer": true},
                                ],
                                "readings": [
                                    {
                                        "type": "type1",
                                        "primary": true,
                                        "reading": "a",
                                        "accepted_answer": true,
                                    },
                                    {
                                        "type": "type1",
                                        "primary": false,
                                        "reading": "b",
                                        "accepted_answer": true,
                                    },
                                    {
                                        "type": "type2",
                                        "primary": false,
                                        "reading": "c",
                                        "accepted_answer": true,
                                    },
                                    {
                                        "type": "type2",
                                        "primary": false,
                                        "reading": "d",
                                        "accepted_answer": false,
                                    },
                                ],
                            },
                        },
                    ],
                    "pages": {
                        "next_url": format!("{}/subjects?types=vocabulary&hidden=false&page_after_id=1", client.base_url),
                    },
                })
                .to_string(),
            )
            .create();
        let _page2 = mock("GET", "/subjects")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("types".into(), "vocabulary".into()),
                Matcher::UrlEncoded("hidden".into(), "false".into()),
                Matcher::UrlEncoded("page_after_id".into(), "1".into()),
            ]))
            .with_status(200)
            .with_body(
                json!({
                    "data": [
                        {
                            "id": 2,
                            "object": "vocabulary",
                            "data": {
                                "document_url": "https://www.wanikani.com/vocabulary/b",
                                "characters": "b",
                                "meanings": [
                                    {"meaning": "b", "primary": true, "accepted_answer": true},
                                ],
                                "readings": [
                                    {
                                        "type": "type1",
                                        "primary": true,
                                        "reading": "b",
                                        "accepted_answer": true,
                                    },
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

        assert_eq!(
            client.vocabulary().await?,
            vec![
                Vocabulary {
                    id: 1,
                    document_url: "https://www.wanikani.com/vocabulary/a".to_string(),
                    characters: "a".to_string(),
                    meanings: vec!["a1".to_string(), "a3".to_string()],
                    readings: vec!["a".to_string(), "b".to_string(), "c".to_string()],
                },
                Vocabulary {
                    id: 2,
                    document_url: "https://www.wanikani.com/vocabulary/b".to_string(),
                    characters: "b".to_string(),
                    meanings: vec!["b".to_string()],
                    readings: vec!["b".to_string()],
                },
            ]
        );

        Ok(())
    }
}
