use std::{fs::read_to_string, path::Path};

use chrono::Utc;
use ideas::{post_page, Post, Site};

fn main() -> anyhow::Result<()> {
    let site = Site {
        name: "ideas".into(),
        base_url: "/ideas".into(),
        now: Utc::now(),
    };
    let path = Path::new("default/1638411458.md");
    let post = Post::new(
        path.iter()
            .map(|part| part.to_str().unwrap().into())
            .collect(),
        &read_to_string(Path::new("content").join(path))?,
    )?;
    println!("{}", post_page(&site, &post, None, None));
    Ok(())
}
