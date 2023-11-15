use std::{
    fs::{copy, create_dir_all, read_dir, read_to_string, remove_dir_all, write},
    path::Path,
};

use anyhow::Context;
use chrono::Utc;
use ideas::{compile_scss, post_page, Post, Site};
use walkdir::WalkDir;

fn main() -> anyhow::Result<()> {
    let site = Site {
        name: "ideas".into(),
        base_url: "/ideas".into(),
        now: Utc::now(),
    };

    let content_dir = Path::new("content");
    let resource_dir = Path::new("resources");
    let build_dir = Path::new("target/web/ideas");
    create_dir_all(build_dir)?;
    remove_dir_all(build_dir)?;
    create_dir_all(build_dir)?;

    create_dir_all(build_dir.join("assets"))?;
    for entry in WalkDir::new(resource_dir.join("assets")) {
        let entry = entry?;
        let target = build_dir.join(entry.path().strip_prefix(resource_dir).unwrap());
        if entry.file_type().is_file() {
            copy(entry.path(), &target).context(entry.path().display().to_string())?;
        }
        if entry.file_type().is_dir() {
            create_dir_all(&target)?
        }
    }
    write(build_dir.join("assets").join("main.css"), compile_scss()?)?;

    let mut posts = read_dir(content_dir.join("default"))?
        .map(|entry| {
            let path = entry?.path();
            Ok(Post::new(
                path.strip_prefix(content_dir)
                    .unwrap()
                    .with_extension("")
                    .into_iter()
                    .map(|component| component.to_str().unwrap().into())
                    .collect(),
                &read_to_string(path)?,
            )?)
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    posts.sort_unstable_by_key(|post| post.path.clone());
    if let Some(post) = posts.first() {
        let page = post_page(&site, post, None, posts.iter().nth(1));
        write_post_page(build_dir, &post.path, &page)?
    }
    for window in posts.windows(3) {
        let [prev_post, post, next_post] = window else {
            unreachable!()
        };
        let page = post_page(&site, post, Some(prev_post), Some(next_post));
        write_post_page(build_dir, &post.path, &page)?
    }
    let mut posts_rev = posts.iter().rev();
    if let (Some(post), Some(prev_post)) = (posts_rev.next(), posts_rev.next()) {
        let page = post_page(&site, post, Some(prev_post), None);
        write_post_page(build_dir, &post.path, &page)?
    }

    Ok(())
}

fn write_post_page(build_dir: &Path, path: &[String], page: &str) -> anyhow::Result<()> {
    let target_dir = build_dir.join(path.join("/"));
    create_dir_all(&target_dir)?;
    write(target_dir.join("index.html"), page)?;
    Ok(())
}
