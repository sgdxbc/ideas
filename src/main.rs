fn main() {
    let content = std::fs::read_to_string("_drafts/001.md").unwrap();
    println!("{:?}", content.parse::<ideas::Frontmatter>());
}
