use chrono::{DateTime, FixedOffset};
use serde::Serialize;

#[derive(Serialize, Debug, PartialEq, Eq, Clone)]
pub enum Subject {
    Radical(Radical),
    Kanji(Kanji),
    Vocabulary(Vocabulary),
}

#[derive(Serialize, Debug, PartialEq, Eq, Clone)]
pub struct Radical {
    pub id: u64,
    pub document_url: String,
    pub characters: Option<String>,
    pub character_svg_path: Option<String>,
    pub meanings: Vec<String>,
}

#[derive(Serialize, Debug, PartialEq, Eq, Clone)]
pub struct Kanji {
    pub id: u64,
    pub document_url: String,
    pub characters: String,
    pub meanings: Vec<String>,
    pub readings: Vec<String>,
}

#[derive(Serialize, Debug, PartialEq, Eq, Clone)]
pub struct Vocabulary {
    pub id: u64,
    pub document_url: String,
    pub characters: String,
    pub meanings: Vec<String>,
    pub readings: Vec<String>,
}

#[derive(Serialize, Debug, PartialEq, Eq, Clone)]
pub struct Assignment {
    pub subject: Subject,
    pub srs_stage: u64,
    pub available_at: DateTime<FixedOffset>,
}
