use std::{
    fs::{copy, create_dir_all, read_dir, read_to_string, remove_dir_all, write},
    path::Path,
};

use anyhow::Context;
use chrono::Utc;
use ideas::{compile_scss, home_page, Catalogue, Post, Site};
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
        let page = post.render(&site, None, posts.iter().nth(1));
        write_page(build_dir, &post.path, &page)?
    }
    for window in posts.windows(3) {
        let [prev_post, post, next_post] = window else {
            unreachable!()
        };
        let page = post.render(&site, Some(prev_post), Some(next_post));
        write_page(build_dir, &post.path, &page)?
    }
    let mut posts_rev = posts.iter().rev();
    if let (Some(post), Some(prev_post)) = (posts_rev.next(), posts_rev.next()) {
        let page = post.render(&site, Some(prev_post), None);
        write_page(build_dir, &post.path, &page)?
    }

    let catalogues = posts
        .chunks(20)
        .enumerate()
        .map(|(page_num, posts)| Catalogue {
            posts: posts.iter().collect(),
            path: vec!["default".into(), "page".into(), page_num.to_string()],
        })
        .collect::<Vec<_>>();
    if let Some(catelogue) = catalogues.first() {
        let page = catelogue.render(&site, None, catalogues.iter().nth(1));
        write_page(build_dir, &catelogue.path, &page)?
    }
    for window in catalogues.windows(3) {
        let [prev_catalogue, catalogue, next_catalogue] = window else {
            unreachable!()
        };
        let page = catalogue.render(&site, Some(prev_catalogue), Some(next_catalogue));
        write_page(build_dir, &catalogue.path, &page)?
    }
    let mut catalogues_rev = catalogues.iter().rev();
    if let (Some(catalogue), Some(prev_catalogue)) = (catalogues_rev.next(), catalogues_rev.next())
    {
        let page = catalogue.render(&site, Some(prev_catalogue), None);
        write_page(build_dir, &catalogue.path, &page)?
    }

    let post = posts.iter().max_by_key(|post| post.date);
    let page = home_page(&site, post);
    write_page(build_dir, &[], &page)?;

    Ok(())
}

fn write_page(build_dir: &Path, path: &[String], page: &str) -> anyhow::Result<()> {
    let target_dir = build_dir.join(path.join("/"));
    create_dir_all(&target_dir)?;
    write(target_dir.join("index.html"), page)?;
    Ok(())
}
