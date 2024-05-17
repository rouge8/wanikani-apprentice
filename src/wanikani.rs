use std::collections::HashMap;
use std::time::Instant;

use anyhow::{bail, Result};
use chrono::DateTime;
use serde_json::Value;
use tracing::info;

use crate::db::Database;
use crate::models::{Assignment, KanaVocabulary, Kanji, Radical, Subject, Vocabulary};

pub struct WaniKaniAPIClient<'a> {
    pub base_url: String,
    api_key: String,
    client: &'a reqwest::Client,
}

#[derive(strum_macros::Display)]
#[strum(serialize_all = "snake_case")]
enum SubjectType {
    Radical,
    Kanji,
    Vocabulary,
    KanaVocabulary,
}

const APPRENTICE_SRS_STAGES: [u8; 4] = [1, 2, 3, 4];

impl<'a> WaniKaniAPIClient<'a> {
    pub fn new(api_key: &str, base_url: &str, client: &'a reqwest::Client) -> Self {
        Self {
            base_url: base_url.to_string(),
            api_key: api_key.to_string(),
            client,
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

    pub async fn kana_vocabulary(&self) -> reqwest::Result<Vec<KanaVocabulary>> {
        let mut results = Vec::new();

        for kana_vocab in self.subjects(SubjectType::KanaVocabulary).await? {
            results.push(KanaVocabulary {
                id: kana_vocab["id"].as_u64().unwrap(),
                document_url: kana_vocab["data"]["document_url"]
                    .as_str()
                    .unwrap()
                    .to_string(),
                characters: kana_vocab["data"]["characters"]
                    .as_str()
                    .unwrap()
                    .to_string(),
                meanings: kana_vocab["data"]["meanings"]
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

    pub async fn assignments(&self, db: &Database) -> Result<Vec<Assignment>> {
        let mut results = Vec::new();

        let apprentice_srs_stages = APPRENTICE_SRS_STAGES
            .map(|stage| stage.to_string())
            .join(",");
        let params = HashMap::from([
            ("srs_stages", apprentice_srs_stages.as_str()),
            ("hidden", "false"),
        ]);
        // TODO: Handle possible (but unlikely) pagination
        let resp: Value = self
            .request("assignments", Some(&params))
            .await?
            .json()
            .await?;

        for assignment in resp["data"].as_array().unwrap() {
            let subject_id = assignment["data"]["subject_id"].as_u64().unwrap();
            let subject_type = assignment["data"]["subject_type"].as_str().unwrap();

            let subject = match subject_type {
                "radical" => match db.radical.get(&subject_id) {
                    Some(radical) => Subject::Radical(radical.clone()),
                    None => bail!("Unknown radical: {}", &subject_id),
                },
                "kanji" => match db.kanji.get(&subject_id) {
                    Some(kanji) => Subject::Kanji(kanji.clone()),
                    None => bail!("Unknown kanji: {}", &subject_id),
                },
                "vocabulary" => match db.vocabulary.get(&subject_id) {
                    Some(vocabulary) => Subject::Vocabulary(vocabulary.clone()),
                    None => bail!("Unknown vocabulary: {}", &subject_id),
                },
                "kana_vocabulary" => match db.kana_vocabulary.get(&subject_id) {
                    Some(kana_vocabulary) => Subject::KanaVocabulary(kana_vocabulary.clone()),
                    None => bail!("Unknown kana_vocabulary: {}", &subject_id),
                },
                _ => bail!("Unknown subject type: {}", &subject_type),
            };

            results.push(Assignment {
                subject,
                srs_stage: assignment["data"]["srs_stage"].as_u64().unwrap(),
                available_at: DateTime::parse_from_rfc3339(
                    assignment["data"]["available_at"].as_str().unwrap(),
                )
                .unwrap(),
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
    use anyhow::anyhow;
    use mockito::Matcher;
    use once_cell::sync::OnceCell;
    use rstest::{fixture, rstest};
    use serde_json::json;
    use similar_asserts::assert_eq;

    use super::*;

    static HTTP_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();

    #[fixture]
    async fn mockito_server() -> mockito::ServerGuard {
        mockito::Server::new_async().await
    }

    fn test_client(server: &mockito::ServerGuard) -> WaniKaniAPIClient<'static> {
        WaniKaniAPIClient::new(
            "fake-api-key",
            &server.url(),
            HTTP_CLIENT.get_or_init(reqwest::Client::new),
        )
    }

    #[rstest]
    #[tokio::test]
    async fn test_username(#[future] mockito_server: mockito::ServerGuard) -> reqwest::Result<()> {
        let mut mockito_server = mockito_server.await;
        let client = test_client(&mockito_server);
        let _m = mockito_server
            .mock("GET", "/user")
            .with_status(200)
            .with_body(r#"{"data": {"username": "test-user"}}"#)
            .create_async()
            .await;

        assert_eq!(client.username().await?, "test-user");

        Ok(())
    }

    #[rstest]
    #[tokio::test]
    async fn test_radicals(#[future] mockito_server: mockito::ServerGuard) -> reqwest::Result<()> {
        let mut mockito_server = mockito_server.await;
        let client = test_client(&mockito_server);
        let _m = mockito_server.mock("GET", "/subjects")
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
            .create_async()
            .await;

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

    #[rstest]
    #[tokio::test]
    async fn test_radicals_with_character_images(
        #[future] mockito_server: mockito::ServerGuard,
    ) -> reqwest::Result<()> {
        let mut mockito_server = mockito_server.await;
        let client = test_client(&mockito_server);
        let _m = mockito_server
            .mock("GET", "/subjects")
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
            .create_async()
            .await;

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

    #[rstest]
    #[tokio::test]
    async fn test_kanji(#[future] mockito_server: mockito::ServerGuard) -> reqwest::Result<()> {
        let mut mockito_server = mockito_server.await;
        let client = test_client(&mockito_server);
        let _page1 = mockito_server.mock("GET", "/subjects")
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
            .create_async()
            .await;
        let _page2 = mockito_server
            .mock("GET", "/subjects")
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
            .create_async()
            .await;

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

    #[rstest]
    #[tokio::test]
    async fn test_vocabulary(
        #[future] mockito_server: mockito::ServerGuard,
    ) -> reqwest::Result<()> {
        let mut mockito_server = mockito_server.await;
        let client = test_client(&mockito_server);
        let _page1 = mockito_server.mock("GET", "/subjects")
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
            .create_async()
            .await;
        let _page2 = mockito_server
            .mock("GET", "/subjects")
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
            .create_async()
            .await;

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

    #[rstest]
    #[tokio::test]
    async fn test_kana_vocabulary(
        #[future] mockito_server: mockito::ServerGuard,
    ) -> reqwest::Result<()> {
        let mut mockito_server = mockito_server.await;
        let client = test_client(&mockito_server);
        let _page1 = mockito_server.mock("GET", "/subjects")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("types".into(), "kana_vocabulary".into()),
                Matcher::UrlEncoded("hidden".into(), "false".into()),
            ]))
            .with_status(200)
            .with_body(
                json!({
                    "data": [
                        {
                            "id": 1,
                            "object": "kana_vocabulary",
                            "data": {
                                "document_url": "https://www.wanikani.com/vocabulary/a",
                                "characters": "a",
                                "meanings": [
                                    {"meaning": "a1", "primary": true, "accepted_answer": true},
                                    {"meaning": "a2", "primary": false, "accepted_answer": false},
                                    {"meaning": "a3", "primary": false, "accepted_answer": true},
                                ],
                            },
                        },
                    ],
                    "pages": {
                        "next_url": format!("{}/subjects?types=kana_vocabulary&hidden=false&page_after_id=1", client.base_url),
                    },
                })
                .to_string(),
            )
            .create_async()
            .await;
        let _page2 = mockito_server
            .mock("GET", "/subjects")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("types".into(), "kana_vocabulary".into()),
                Matcher::UrlEncoded("hidden".into(), "false".into()),
                Matcher::UrlEncoded("page_after_id".into(), "1".into()),
            ]))
            .with_status(200)
            .with_body(
                json!({
                    "data": [
                        {
                            "id": 2,
                            "object": "kana_vocabulary",
                            "data": {
                                "document_url": "https://www.wanikani.com/vocabulary/b",
                                "characters": "b",
                                "meanings": [
                                    {"meaning": "b", "primary": true, "accepted_answer": true},
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
            .create_async()
            .await;

        assert_eq!(
            client.kana_vocabulary().await?,
            vec![
                KanaVocabulary {
                    id: 1,
                    document_url: "https://www.wanikani.com/vocabulary/a".to_string(),
                    characters: "a".to_string(),
                    meanings: vec!["a1".to_string(), "a3".to_string()],
                },
                KanaVocabulary {
                    id: 2,
                    document_url: "https://www.wanikani.com/vocabulary/b".to_string(),
                    characters: "b".to_string(),
                    meanings: vec!["b".to_string()],
                },
            ]
        );

        Ok(())
    }

    #[rstest]
    #[tokio::test]
    async fn test_assignments(#[future] mockito_server: mockito::ServerGuard) -> Result<()> {
        let mut mockito_server = mockito_server.await;
        let client = test_client(&mockito_server);
        let _m = mockito_server
            .mock("GET", "/assignments")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("srs_stages".into(), "1,2,3,4".into()),
                Matcher::UrlEncoded("hidden".into(), "false".into()),
            ]))
            .with_status(200)
            .with_body(
                json!({
                    "data": [
                        {
                            "id": 1,
                            "object": "assignment",
                            "data": {
                                "subject_id": 1,
                                "subject_type": "radical",
                                "srs_stage": 1,
                                "available_at": "2022-07-11T16:00:00.000000Z",
                            },
                        },
                        {
                            "id": 2,
                            "object": "assignment",
                            "data": {
                                "subject_id": 2,
                                "subject_type": "kanji",
                                "srs_stage": 2,
                                "available_at": "2022-07-16T21:00:00.000000Z",
                            },
                        },
                        {
                            "id": 3,
                            "object": "assignment",
                            "data": {
                                "subject_id": 3,
                                "subject_type": "vocabulary",
                                "srs_stage": 3,
                                "available_at": "2022-07-15T14:00:00.000000Z",
                            },
                        },
                        {
                            "id": 4,
                            "object": "assignment",
                            "data": {
                                "subject_id": 4,
                                "subject_type": "kana_vocabulary",
                                "srs_stage": 4,
                                "available_at": "2022-07-16T14:00:00.000000Z",
                            },
                        },
                    ],
                })
                .to_string(),
            )
            .create();

        let radical = Radical {
            id: 1,
            document_url: "https://www.wanikani.com/radicals/before".to_string(),
            characters: Some("前".to_string()),
            character_svg_path: None,
            meanings: vec!["before".to_string()],
        };
        let kanji = Kanji {
            id: 2,
            document_url: "https://www.wanikani.com/kanji/a".to_string(),
            characters: "a".to_string(),
            meanings: vec!["a".to_string()],
            readings: vec!["a".to_string()],
        };
        let vocabulary = Vocabulary {
            id: 3,
            document_url: "https://www.wanikani.com/vocabulary/魚".to_string(),
            characters: "魚".to_string(),
            meanings: vec!["fish".to_string()],
            readings: vec!["さかな".to_string()],
        };
        let kana_vocabulary = KanaVocabulary {
            id: 4,
            document_url: "https://www.wanikani.com/vocabulary/リンゴ".to_string(),
            characters: "リンゴ".to_string(),
            meanings: vec!["apple".to_string()],
        };

        let mut db = Database::new();
        db.radical.insert(1, radical.clone());
        db.kanji.insert(2, kanji.clone());
        db.vocabulary.insert(3, vocabulary.clone());
        db.kana_vocabulary.insert(4, kana_vocabulary.clone());

        assert_eq!(
            client.assignments(&db).await?,
            vec![
                Assignment {
                    subject: Subject::Radical(radical),
                    srs_stage: 1,
                    available_at: DateTime::parse_from_rfc3339("2022-07-11T16:00:00.000000Z")
                        .unwrap(),
                },
                Assignment {
                    subject: Subject::Kanji(kanji),
                    srs_stage: 2,
                    available_at: DateTime::parse_from_rfc3339("2022-07-16T21:00:00.000000Z")
                        .unwrap(),
                },
                Assignment {
                    subject: Subject::Vocabulary(vocabulary),
                    srs_stage: 3,
                    available_at: DateTime::parse_from_rfc3339("2022-07-15T14:00:00.000000Z")
                        .unwrap(),
                },
                Assignment {
                    subject: Subject::KanaVocabulary(kana_vocabulary),
                    srs_stage: 4,
                    available_at: DateTime::parse_from_rfc3339("2022-07-16T14:00:00.000000Z")
                        .unwrap(),
                },
            ]
        );

        Ok(())
    }

    #[rstest]
    #[case("radical")]
    #[case("kanji")]
    #[case("vocabulary")]
    #[case("kana_vocabulary")]
    #[tokio::test]
    async fn test_assignments_unknown_subject(
        #[case] subject_type: &str,
        #[future] mockito_server: mockito::ServerGuard,
    ) -> Result<()> {
        let mut mockito_server = mockito_server.await;
        let client = test_client(&mockito_server);
        let _m = mockito_server
            .mock("GET", "/assignments")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("srs_stages".into(), "1,2,3,4".into()),
                Matcher::UrlEncoded("hidden".into(), "false".into()),
            ]))
            .with_status(200)
            .with_body(
                json!({
                    "data": [
                        {
                            "id": 1,
                            "object": "assignment",
                            "data": {
                                "subject_id": 1,
                                "subject_type": subject_type,
                                "srs_stage": 1,
                                "available_at": "2022-07-11T16:00:00.000000Z",
                            },
                        },
                    ],
                })
                .to_string(),
            )
            .create();

        let db = Database::new();

        assert_eq!(
            client.assignments(&db).await.unwrap_err().to_string(),
            anyhow!("Unknown {}: 1", subject_type).to_string(),
        );

        Ok(())
    }
}
