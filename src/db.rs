use std::collections::HashMap;

use tracing::info;

use crate::models::{Kanji, Radical, Vocabulary};
use crate::wanikani::WaniKaniAPIClient;

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

    pub async fn populate(&mut self, api: &WaniKaniAPIClient<'_>) -> reqwest::Result<()> {
        let result = tokio::try_join!(
            Self::get_radicals(api),
            Self::get_kanji(api),
            Self::get_vocabulary(api),
        )?;

        let (radicals, kanji, vocabulary) = result;
        self.radical.extend(radicals);
        self.kanji.extend(kanji);
        self.vocabulary.extend(vocabulary);

        Ok(())
    }

    async fn get_radicals(api: &WaniKaniAPIClient<'_>) -> reqwest::Result<HashMap<u64, Radical>> {
        let mut result = HashMap::new();

        for radical in api.radicals().await? {
            result.insert(radical.id, radical);
        }
        info!(n = result.len(), "loaded radicals");

        Ok(result)
    }

    async fn get_kanji(api: &WaniKaniAPIClient<'_>) -> reqwest::Result<HashMap<u64, Kanji>> {
        let mut result = HashMap::new();

        for kanji in api.kanji().await? {
            result.insert(kanji.id, kanji);
        }
        info!(n = result.len(), "loaded kanji");

        Ok(result)
    }

    async fn get_vocabulary(
        api: &WaniKaniAPIClient<'_>,
    ) -> reqwest::Result<HashMap<u64, Vocabulary>> {
        let mut result = HashMap::new();

        for vocabulary in api.vocabulary().await? {
            result.insert(vocabulary.id, vocabulary);
        }
        info!(n = result.len(), "loaded vocabulary");

        Ok(result)
    }
}
