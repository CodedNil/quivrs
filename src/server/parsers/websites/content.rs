use super::{PageData, jsonld_values};
use dom_smoothie::Readability;
use scraper::{Html, Selector};
use serde_json::Value;
use std::sync::LazyLock;

static SEL_ARTICLE: LazyLock<Selector> = LazyLock::new(|| Selector::parse("article").unwrap());
static SEL_ARTICLE_BLOCK: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(r"div[data-block], div[data-component]").unwrap());
static SEL_ARTICLE_TEXT_BLOCK: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse(
        r#"div[data-block="text"], div[data-block="headline"], div[data-block="subheadline"], div[data-component="text-block"], div[data-component="headline-block"]"#,
    )
    .unwrap()
});
static SEL_TEXT_CONTENT: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("p, h1, h2, h3").unwrap());

pub fn parse(page: &PageData, url: &str) -> Option<String> {
    if let Some(content) = page
        .jsonld
        .iter()
        .flat_map(jsonld_values)
        .find_map(|obj| obj.get("articleBody").and_then(Value::as_str))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
    {
        return Some(content);
    }

    let doc = Html::parse_document(&page.html);
    let mut content = String::new();

    if let Some(article) = doc.select(&SEL_ARTICLE).next() {
        for block in article.select(&SEL_ARTICLE_BLOCK) {
            if SEL_ARTICLE_TEXT_BLOCK.matches(&block) {
                for el in block.select(&SEL_TEXT_CONTENT) {
                    let text = el.text().collect::<Vec<_>>().join(" ");
                    if !text.trim().is_empty() {
                        content.push_str(&text);
                        content.push_str("\n\n");
                    }
                }
            }
        }
    }

    if !content.trim().is_empty() {
        return Some(content.trim().to_string());
    }

    Readability::new(page.html.as_str(), Some(url), None)
        .ok()
        .and_then(|mut readability| readability.parse().ok())
        .map(|article| article.text_content.to_string())
        .filter(|content| !content.is_empty())
}
