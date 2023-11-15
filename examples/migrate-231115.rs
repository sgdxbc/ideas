use std::{
    fs::{create_dir_all, read_to_string, write, File},
    io::Write,
    path::Path,
};

use anyhow::Context;
use ideas::Frontmatter;

fn main() -> anyhow::Result<()> {
    migrate_directory("_posts".as_ref(), |s| {
        s.rsplit_once('-').unwrap().1.to_string()
    })?;
    migrate_directory("_drafts".as_ref(), ToString::to_string)?;
    Ok(())
}

fn migrate_directory(dir: &Path, mut mapper: impl FnMut(&str) -> String) -> anyhow::Result<()> {
    let content_dir = Path::new("content");
    let dest_dir = content_dir.join("default");
    create_dir_all(&dest_dir)?;
    let mut redirect = File::options()
        .create(true)
        .append(true)
        .open(content_dir.join("redirect.txt"))?;
    for file in std::fs::read_dir(dir)? {
        let path = file?.path();
        let content = read_to_string(&path)?;
        let date = content
            .parse::<Frontmatter>()?
            .date
            .ok_or(anyhow::anyhow!("no date in frontmatter"))
            .context(format!("{}", path.display()))?;
        write(
            dest_dir
                .join(date.timestamp().to_string())
                .with_extension(path.extension().unwrap_or_default()),
            &content,
        )?;
        writeln!(
            redirect,
            "{}/{} => {}",
            date.format("%F"),
            mapper(
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or(anyhow::anyhow!("cannot get file stem"))?
            ),
            date.timestamp()
        )?
    }
    Ok(())
}
