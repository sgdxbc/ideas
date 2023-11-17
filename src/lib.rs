use std::{mem::replace, rc::Rc, str::FromStr};

use anyhow::Context;
use chrono::{DateTime, FixedOffset, LocalResult, NaiveDate, NaiveDateTime, Utc};
use html5ever::{tendril::TendrilSink, QualName};
use markdown::Options;
use markup5ever::{local_name, namespace_url, ns, Attribute};
use markup5ever_rcdom::{Node, NodeData, RcDom};
use maud::{Markup, PreEscaped};
use rsass::output::{Format, Style::Compressed};
use serde::Deserialize;

pub fn compile_scss() -> anyhow::Result<Vec<u8>> {
    Ok(rsass::compile_scss_path(
        "resources/sass/tale.scss".as_ref(),
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
    if let Ok(date) = DateTime::parse_from_rfc2822(date) {
        return Ok(date);
    }
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
    pub path: Vec<String>,
    pub date: DateTime<FixedOffset>,
    title: Option<String>,
    body: String,         // compiled
    body_excerpt: String, // compiled
}

impl Post {
    pub fn new(path: Vec<String>, content: &str) -> anyhow::Result<Self> {
        // println!("{path:?}");
        let frontmatter = content.parse::<Frontmatter>()?;
        let mut body_dom = html5ever::parse_document(RcDom::default(), Default::default())
            .from_utf8()
            .read_from(&mut compile_markdown(content).as_bytes())?;
        Self::walk_markdown_node(&mut body_dom.document);
        let mut body = Vec::new();
        html5ever::serialize(
            &mut body,
            &markup5ever_rcdom::SerializableHandle::from(body_dom.document.clone()),
            Default::default(),
        )?;
        Self::make_excerpt(&mut body_dom.document);
        let mut body_excerpt = Vec::new();
        html5ever::serialize(
            &mut body_excerpt,
            &markup5ever_rcdom::SerializableHandle::from(body_dom.document.clone()),
            Default::default(),
        )?;
        Ok(Self {
            path,
            date: frontmatter
                .date
                .ok_or(anyhow::anyhow!("missing date in frontmatter"))?,
            title: frontmatter.title,
            body: String::from_utf8(body)?,
            body_excerpt: String::from_utf8(body_excerpt)?,
        })
    }

    fn walk_markdown_node(node: &mut Rc<Node>) {
        if let NodeData::Element { name, attrs, .. } = &node.data {
            if name == &QualName::new(None, ns!(html), "pre".into()) {
                let attr = Attribute {
                    name: QualName::new(None, ns!(), "class".into()),
                    value: "highlight".into(),
                };
                attrs.borrow_mut().push(attr.clone());
                let pre_node = replace(
                    node,
                    Node::new(NodeData::Element {
                        name: QualName::new(None, ns!(html), "div".into()),
                        attrs: vec![attr].into(),
                        template_contents: None.into(),
                        mathml_annotation_xml_integration_point: false,
                    }),
                );
                node.children.borrow_mut().push(pre_node);
                return;
            }
        }
        for child in node.children.borrow_mut().iter_mut() {
            Self::walk_markdown_node(child)
        }
    }

    fn canonical_title(&self) -> String {
        if let Some(title) = &self.title {
            title.into()
        } else {
            self.path.last().unwrap().into()
        }
    }

    fn make_excerpt(node: &Node) {
        let mut more_comment = None;
        for (index, child) in node.children.borrow().iter().enumerate() {
            if let NodeData::Element { name, .. } = &child.data {
                // println!("{name:?}");
                if matches!(name.local, local_name!("html")) {
                    let children = child.children.borrow();
                    assert_eq!(children.len(), 2);
                    return Self::make_excerpt(children.last().unwrap());
                }
                if matches!(name.local, local_name!("body")) {
                    let children = child.children.borrow();
                    assert_eq!(children.len(), 1);
                    return Self::make_excerpt(children.last().unwrap());
                }
            }

            if let NodeData::Comment { contents } = &child.data {
                if contents.trim() == "more" {
                    more_comment = Some(index);
                    break;
                }
            }
        }
        node.children
            .borrow_mut()
            .truncate(more_comment.unwrap_or(1))
    }

    pub fn render(
        &self,
        site: &Site,
        prev_post: Option<&Self>,
        next_post: Option<&Self>,
    ) -> String {
        default_page(
            site,
            &format!("{} \u{00bb} {}", site.name, self.canonical_title()),
            maud::html! {
                .post {
                    .post-info {
                        time datetime={ (self.date.to_rfc3339()) } { (self.date.to_rfc2822()) }
                    }
                    @if let Some(title) = &self.title {
                        h1 .post-title { (title) }
                    }
                    @else {
                        .post-title {}
                    }
                    .post-line {}
                    (PreEscaped(&self.body))
                }
                // TODO post title
                .pagination {
                    @if let Some(prev_post) = prev_post {
                        a href={ (site.base_url) "/" (prev_post.path.join("/")) } .left .arrow {
                            (PreEscaped("&#8592;"))
                        }
                    }
                    @if let Some(next_post) = next_post {
                        a href={ (site.base_url) "/" (next_post.path.join("/")) } .right .arrow {
                            (PreEscaped("&#8594;"))
                        }
                    }
                    a href="#" .top { "Top" }
                }
            },
        )
        .into()
    }
}

#[derive(Debug)]
pub struct Catalogue<'a> {
    pub path: Vec<String>,
    pub posts: Vec<&'a Post>,
}

impl Catalogue<'_> {
    pub fn render(&self, site: &Site, prev: Option<&Self>, next: Option<&Self>) -> String {
        let mut title = vec!["ideas"];
        title.extend(self.path.iter().map(AsRef::<str>::as_ref));
        default_page(
            site,
            &title.join(" \u{00bb} "),
            maud::html! {
                .catalogue {
                    @for post in &self.posts {
                        a href={ (site.base_url) "/" (post.path.join("/")) } .catalogue-item {
                            time datetime={ (post.date.to_rfc3339()) } .catalogue-time { (post.date.to_rfc2822()) }
                            @if let Some(title) = &post.title {
                                h1 .catalogue-title { (title) }
                            }
                            @else {
                                .catalogue-title {}
                            }
                            .catalogue-line {}
                            (PreEscaped(&post.body_excerpt))
                        }
                    }
                }
                .pagination {
                    @if let Some(prev) = prev {
                        a href={ (site.base_url) "/" (prev.path.join("/")) } .left .arrow {
                            (PreEscaped("&#8592;"))
                        }
                    }
                    @if let Some(next) = next {
                        a href={ (site.base_url) "/" (next.path.join("/")) } .right .arrow {
                            (PreEscaped("&#8594;"))
                        }
                    }
                }
            },
        )
        .into()
    }
}

pub fn home_page(site: &Site, post: Option<&Post>) -> String {
    default_page(
        site,
        "ideas",
        maud::html! {
            @if let Some(post) = post {
                a href={ (site.base_url) "/" (post.path.join("/")) } .catalogue-item {
                    time datetime={ (post.date.to_rfc3339()) } .catalogue-time { (post.date.to_rfc2822()) }
                    @if let Some(title) = &post.title {
                        h1 .catalogue-title { (title) }
                    }
                    @else {
                        .catalogue-title {}
                    }
                    .catalogue-line {}
                    (PreEscaped(&post.body_excerpt))
                }
            }
        },
    ).into()
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
                ul {
                    li {
                        // TODO enumerate catalugues
                        a href={ (site.base_url) "/default/page/0" } { "Posts" }
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
