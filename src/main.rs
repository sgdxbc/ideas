use std::{fs::read_to_string, str::FromStr, time::SystemTime};

use chrono::DateTime;
use markdown::Options;
use rsass::output::{Format, Style::Compressed};
use serde::Deserialize;

fn main() {
    let css = rsass::compile_scss_path(
        "_sass/tale.scss".as_ref(),
        Format {
            style: Compressed,
            ..Default::default()
        },
    )
    .unwrap();
    println!("{}", css.len());

    let content = read_to_string("_drafts/001.md").unwrap();
    let mut options = Options::gfm();
    options.parse.constructs.frontmatter = true;
    let md = markdown::to_html_with_options(&content, &options).unwrap();
    println!("{md}");
    println!("{:?}", content.parse::<Frontmatter>());
}

#[derive(Debug)]
struct Frontmatter {
    date: Option<SystemTime>,
    title: Option<String>,
}

impl FromStr for Frontmatter {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[derive(Deserialize)]
        struct Layout {
            date: Option<String>,
            title: Option<String>,
        }
        let mut lines = s.lines();
        if lines.next() != Some("---") {
            anyhow::bail!("missing frontmatter")
        }
        let raw = lines
            .take_while(|line| *line != "---")
            .collect::<Vec<_>>()
            .join("\n");
        let layout = serde_yaml::from_str::<Layout>(&raw)?;
        Ok(Self {
            date: layout
                .date
                .map(|date| DateTime::parse_from_str(&date, "%F %R %z"))
                .transpose()?
                .map(Into::into),
            title: layout.title,
        })
    }
}
