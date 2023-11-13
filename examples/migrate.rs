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
    let dest_dir = Path::new("content");
    create_dir_all(dest_dir)?;
    let migrated_dir = Path::new("migrated");
    create_dir_all(migrated_dir.join(dir))?;
    let mut redirect = File::options()
        .create(true)
        .append(true)
        .open(migrated_dir.join("redirect.txt"))?;
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
                .join(date.to_rfc3339())
                .with_extension(path.extension().unwrap_or_default()),
            &content,
        )?;
        write(migrated_dir.join(&path), content)?;
        writeln!(
            redirect,
            "{}/{} => {}",
            date.format("%F"),
            mapper(
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or(anyhow::anyhow!("cannot get file stem"))?
            ),
            date.to_rfc3339()
        )?
    }
    Ok(())
}
