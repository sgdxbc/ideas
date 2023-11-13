use std::{
    fs::{create_dir_all, read_to_string, write},
    path::Path,
};

use anyhow::Context;
use ideas::Frontmatter;

fn main() -> anyhow::Result<()> {
    migrate_directory("_posts".as_ref())?;
    migrate_directory("_drafts".as_ref())?;
    Ok(())
}

fn migrate_directory(dir: &Path) -> anyhow::Result<()> {
    let dest_dir = Path::new("content");
    create_dir_all(dest_dir)?;
    for file in std::fs::read_dir(dir)? {
        let path = file?.path();
        let content = read_to_string(&path)?;
        let frontmatter = content.parse::<Frontmatter>()?;
        write(
            dest_dir.join(
                frontmatter
                    .date
                    .ok_or(anyhow::anyhow!("no date in frontmatter"))
                    .context(format!("{}", path.display()))?
                    .to_rfc3339(),
            ),
            content,
        )?;
    }
    Ok(())
}
