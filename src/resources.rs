use crate::display_time_remaining;
use include_dir::{include_dir, Dir};
use once_cell::sync::Lazy;
use tera::Tera;

pub static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");

static TEMPLATES_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");
pub static TEMPLATES: Lazy<Tera> = Lazy::new(|| {
    let mut tera = Tera::default();
    match tera.add_raw_templates(
        TEMPLATES_DIR
            .files()
            .map(|file| (file.path().to_string_lossy(), file.contents_utf8().unwrap()))
            .collect::<Vec<_>>(),
    ) {
        Ok(()) => (),
        Err(err) => panic!("Parsing error: {}", err),
    }
    tera.register_filter("display_time_remaining", display_time_remaining);
    tera
});
