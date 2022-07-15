use once_cell::sync::Lazy;
use regex::Regex;

use crate::resources::STATIC_DIR;

pub static BS_PRIMARY_COLOR: Lazy<String> = Lazy::new(|| {
    let re = Regex::new(r"--bs-primary:(#[a-f0-9]{6})").unwrap();
    let css = STATIC_DIR
        .get_file("bootstrap.pulse.min.css")
        .unwrap()
        .contents_utf8()
        .unwrap();
    let caps = re.captures(css).unwrap();
    caps.get(1)
        .expect("couldn't find --bs-primary color")
        .as_str()
        .to_string()
});

pub static COOKIE_NAME: &str = "wanikani-api-key";
