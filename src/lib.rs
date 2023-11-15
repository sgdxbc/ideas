use std::str::FromStr;

use anyhow::Context;
use chrono::{DateTime, FixedOffset, LocalResult, NaiveDate, NaiveDateTime, Utc};
use markdown::Options;
use maud::{Markup, PreEscaped};
use rsass::output::{Format, Style::Compressed};
use serde::Deserialize;

pub fn compile_scss() -> anyhow::Result<Vec<u8>> {
    Ok(rsass::compile_scss_path(
        "resource/assets/tale.scss".as_ref(),
        Format {
            style: Compressed,
            ..Default::default()
        },
    )?)
}

pub fn compile_markdown(content: &str) -> String {
    let mut options = Options::gfm();
    options.parse.constructs.frontmatter = true;
    options.compile.allow_dangerous_html = true;
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

#[derive(Debug)]
pub struct Site {
    pub name: String,
    pub base_url: String,
    pub now: DateTime<Utc>,
}

#[derive(Debug)]
pub struct Post {
    path: Vec<String>,
    date: DateTime<FixedOffset>,
    title: Option<String>,
    body: String, // compiled
}

impl Post {
    pub fn new(path: Vec<String>, content: &str) -> anyhow::Result<Self> {
        let frontmatter = content.parse::<Frontmatter>()?;
        Ok(Self {
            path,
            date: frontmatter
                .date
                .ok_or(anyhow::anyhow!("missing date in frontmatter"))?,
            title: frontmatter.title,
            body: compile_markdown(content),
        })
    }

    fn canonical_title(&self) -> String {
        if let Some(title) = &self.title {
            title.into()
        } else {
            self.path.last().unwrap().into()
        }
    }

    fn excerpt(&self) -> String {
        if let Some((excerpt, _)) = self.body.split_once("<!-- more -->") {
            excerpt.into()
        } else {
            self.body.lines().next().unwrap_or_default().into()
        }
    }
}

pub fn post_page(
    site: &Site,
    post: &Post,
    prev_post: Option<&Post>,
    next_post: Option<&Post>,
) -> String {
    default_page(
        site,
        &format!("{} \u{00bb} {}", site.name, post.canonical_title()),
        maud::html! {
            .post {
                .post-info {
                    time datetime={ (post.date.to_rfc3339()) } { (post.date.to_rfc2822()) }
                }
                @if let Some(title) = &post.title {
                    h1 { (title) }
                }
                .post-line {}
                (PreEscaped(&post.body))
            }
            .pagination {
                @if let Some(next_post) = next_post {
                    a href={ (site.base_url) "/" (next_post.path.join("/")) } .left .arrow { "&#8592;" }
                }
                @if let Some(prev_post) = prev_post {
                    a href={ (site.base_url) "/" (prev_post.path.join("/")) } .left .arrow { "&#8594;" }
                }
                a href="#" .top { "Top" }
            }
        },
    )
    .into()
}

fn default_page(site: &Site, title: &str, content: Markup) -> Markup {
    maud::html! {
        (maud::DOCTYPE)
        html {
            (html_head(site, title))
            body {
                (html_navigation(site))
                main { (content) }
                (html_footer(site))
            }
        }
    }
}

fn html_head(site: &Site, title: &str) -> Markup {
    maud::html! {
        head {
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1.0";

            title { (title) }

            link rel="stylesheet" href={ (site.base_url) "/assets/main.css" };
            link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Open+Sans&family=Source+Code+Pro&display=swap";
            link rel="stylesheet" href="https://fonts.googleapis.com/css?family=Libre+Baskerville:400,400i,700";

            link rel="icon" type="image/png" sizes="32x32" href={ (site.base_url) "/assets/favicon-32x32.png" };
            link rel="icon" type="image/png" sizes="16x16" href={ (site.base_url) "/assets/favicon-16x16.png" };
            link rel="apple-touch-icon" sizes="180x180" href={ (site.base_url) "/assets/apple-touch-icon.png" };
        }
    }
}

fn html_navigation(site: &Site) -> Markup {
    maud::html! {
        nav .nav {
            .nav-container {
                a href={ (site.base_url) } {
                    h2 .nav-title { (site.name) }
                }
            }

            ul {
                li {
                    a href={ (site.base_url) } { "Posts" }
                }
                li {
                    a href={ (site.base_url) "/tags" } { del { "Tags" } }
                }
                li {
                    a href={ (site.base_url) "/about" } { "About" }
                }
            }
        }
    }
}

fn html_footer(site: &Site) -> Markup {
    maud::html! {
        footer {
            span {
                "Compiled at "
                time datetime={ (site.now.to_rfc3339()) } { (site.now) }
                ". Made with Rust using a Tale-inspired theme."
            }
        }
    }
}
