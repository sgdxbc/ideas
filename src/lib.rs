use std::str::FromStr;

use anyhow::Context;
use chrono::{DateTime, FixedOffset, LocalResult, NaiveDate, NaiveDateTime};
use markdown::Options;
use rsass::output::{Format, Style::Compressed};
use serde::Deserialize;

pub fn compile_scss() -> anyhow::Result<Vec<u8>> {
    Ok(rsass::compile_scss_path(
        "_sass/tale.scss".as_ref(),
        Format {
            style: Compressed,
            ..Default::default()
        },
    )?)
}

pub fn compile_markdown(content: &str) -> String {
    let mut options = Options::gfm();
    options.parse.constructs.frontmatter = true;
    markdown::to_html_with_options(&content, &options).unwrap()
}

#[derive(Debug)]
pub struct Frontmatter {
    pub date: Option<DateTime<FixedOffset>>,
    pub title: Option<String>,
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
                .map(|date| parse_date(&date).context(date))
                .transpose()?,
            title: layout.title,
        })
    }
}

fn parse_date(date: &str) -> anyhow::Result<DateTime<FixedOffset>> {
    if let Ok(date) = DateTime::parse_from_str(date, "%F %R %z") {
        return Ok(date);
    }
    if let Ok(date) = DateTime::parse_from_str(date, "%F %T %z") {
        return Ok(date);
    }
    let date = if let Ok(date) = NaiveDateTime::parse_from_str(date, "%F %R") {
        date
    } else {
        NaiveDate::parse_from_str(date, "%F")?
            .and_hms_opt(0, 0, 0)
            .unwrap()
    };
    if let LocalResult::Single(date) =
        date.and_local_timezone(FixedOffset::east_opt(8 * 3600).unwrap())
    {
        Ok(date)
    } else {
        anyhow::bail!("fail to upgrade native date time")
    }
}

fn compile_post(content: &str) -> anyhow::Result<String> {
    let meta = content.parse::<Frontmatter>()?;
    let body = compile_markdown(content);
    Ok(Default::default())
}
