use include_dir::{include_dir, Dir};
use minijinja::Environment;
use once_cell::sync::Lazy;

use crate::display_time_remaining;

pub static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");

static TEMPLATES_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");
pub static TEMPLATES: Lazy<Environment> = Lazy::new(|| {
    let mut env = Environment::new();
    let templates = TEMPLATES_DIR
        .files()
        .map(|file| (file.path().to_string_lossy(), file.contents_utf8().unwrap()))
        .collect::<Vec<_>>();
    for (path, template) in templates {
        env.add_template_owned(path, template)
            .expect("Unable to add template");
    }

    env.add_filter("display_time_remaining", display_time_remaining);
    env
});
