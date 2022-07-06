use crate::{
    models::{Kanji, Radical, Vocabulary},
    wanikani::WaniKaniAPIClient,
};
use std::collections::HashMap;
use tracing::info;

pub struct Database {
    pub radical: HashMap<u64, Radical>,
    pub kanji: HashMap<u64, Kanji>,
    pub vocabulary: HashMap<u64, Vocabulary>,
}

impl Database {
    pub fn new() -> Self {
        Self {
            radical: HashMap::new(),
            kanji: HashMap::new(),
            vocabulary: HashMap::new(),
        }
    }

    pub async fn populate(&mut self, api: &WaniKaniAPIClient) -> reqwest::Result<()> {
        let result = tokio::try_join!(
            Self::populate_radicals(api),
            Self::populate_kanji(api),
            Self::populate_vocabulary(api),
        );

        match result {
            Ok((radicals, kanji, vocabulary)) => {
                self.radical.extend(radicals);
                self.kanji.extend(kanji);
                self.vocabulary.extend(vocabulary);
            }
            Err(err) => return Err(err),
        }

        Ok(())
    }

    async fn populate_radicals(api: &WaniKaniAPIClient) -> reqwest::Result<HashMap<u64, Radical>> {
        let mut result = HashMap::new();

        for radical in api.radicals().await? {
            result.insert(radical.id, radical);
        }
        info!(n = result.len(), "loaded radicals");

        Ok(result)
    }

    async fn populate_kanji(api: &WaniKaniAPIClient) -> reqwest::Result<HashMap<u64, Kanji>> {
        let mut result = HashMap::new();

        for kanji in api.kanji().await? {
            result.insert(kanji.id, kanji);
        }
        info!(n = result.len(), "loaded kanji");

        Ok(result)
    }

    async fn populate_vocabulary(
        api: &WaniKaniAPIClient,
    ) -> reqwest::Result<HashMap<u64, Vocabulary>> {
        let mut result = HashMap::new();

        for vocabulary in api.vocabulary().await? {
            result.insert(vocabulary.id, vocabulary);
        }
        info!(n = result.len(), "loaded vocabulary");

        Ok(result)
    }
}
