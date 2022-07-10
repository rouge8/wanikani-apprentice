use std::fs;

use once_cell::sync::Lazy;
use regex::Regex;

pub static BS_PRIMARY_COLOR: Lazy<String> = Lazy::new(|| {
    let re = Regex::new(r"--bs-primary:(#[a-f0-9]{6})").unwrap();
    let css =
        fs::read_to_string("static/bootstrap.pulse.min.css").expect("unable to read CSS file");
    let caps = re.captures(&css).unwrap();
    caps.get(1)
        .expect("couldn't find --bs-primary color")
        .as_str()
        .to_string()
});

pub static COOKIE_NAME: &str = "wanikani-api-key";
