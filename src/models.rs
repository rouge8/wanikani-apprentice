#[derive(Debug, PartialEq, Eq)]
pub struct Radical {
    pub id: u64,
    pub document_url: String,
    pub characters: Option<String>,
    pub character_svg_path: Option<String>,
    pub meanings: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Kanji {
    pub id: u64,
    pub document_url: String,
    pub characters: String,
    pub meanings: Vec<String>,
    pub readings: Vec<String>,
}
