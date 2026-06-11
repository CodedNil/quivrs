use super::{PageData, jsonld_values};
use crate::{server::parsers::normalize_image_url, shared::CaptionedImage};
use itertools::Itertools;
use regex::Regex;
use scraper::{ElementRef, Selector};
use serde_json::Value;
use std::{collections::HashSet, path::Path, sync::LazyLock};
use url::Url;

struct ImageCandidate {
    url: String,
    width: f32,
}

static SEL_IMG: LazyLock<Selector> = LazyLock::new(|| Selector::parse("img").unwrap());
static SEL_PICTURE: LazyLock<Selector> = LazyLock::new(|| Selector::parse("picture").unwrap());
static SEL_SOURCE: LazyLock<Selector> = LazyLock::new(|| Selector::parse("source").unwrap());
static SEL_FIGURE: LazyLock<Selector> = LazyLock::new(|| Selector::parse("figure").unwrap());
static SEL_FIGCAPTION: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("figcaption").unwrap());
static SEL_FIGCAPTION_TEXT: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("figcaption .caption").unwrap());
static RE_RESIZE_WIDTH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"/resize/(\d+)x").unwrap());
static RE_IMAGE_DIMENSIONS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d{3,})x(\d{3,})").unwrap());

pub fn parse(page: &PageData, base: Option<&Url>) -> Vec<CaptionedImage> {
    let mut images = page
        .jsonld
        .iter()
        .flat_map(jsonld_values)
        .filter_map(|obj| obj.get("image").or_else(|| obj.get("thumbnail")))
        .flat_map(|value| structured_images(value, base))
        .collect::<Vec<_>>();

    let doc = scraper::Html::parse_document(&page.html);
    images.extend(extract_images(&doc.root_element(), base));

    dedupe_images(images)
}

pub fn extract_images(root: &ElementRef<'_>, base: Option<&Url>) -> Vec<CaptionedImage> {
    if root.value().name() == "figure" {
        return extract_images_with_caption(root, base, figure_caption(root).as_deref(), false);
    }

    let mut images = root
        .select(&SEL_FIGURE)
        .flat_map(|figure| {
            extract_images_with_caption(&figure, base, figure_caption(&figure).as_deref(), false)
        })
        .collect::<Vec<_>>();

    images.extend(extract_images_with_caption(root, base, None, true));
    images
}

fn structured_images(v: &Value, base: Option<&Url>) -> Vec<CaptionedImage> {
    let mut images = Vec::new();
    collect_structured_images(v, &mut images, base);
    images
}

fn collect_structured_images(v: &Value, out: &mut Vec<CaptionedImage>, base: Option<&Url>) {
    match v {
        Value::Object(obj) => {
            if let (Some(url), Some(caption)) = (
                obj.get("url")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty()),
                obj.get("caption")
                    .and_then(Value::as_str)
                    .filter(|s| !s.trim().is_empty()),
            ) {
                out.push(CaptionedImage {
                    url: resolve_url(base, url),
                    caption: caption.to_string(),
                });
            }
        }
        Value::Array(arr) => arr
            .iter()
            .for_each(|item| collect_structured_images(item, out, base)),
        _ => {}
    }
}

pub fn dedupe_images(mut images: Vec<CaptionedImage>) -> Vec<CaptionedImage> {
    images.sort_by(|a, b| image_url_score(&b.url).total_cmp(&image_url_score(&a.url)));

    let mut seen_urls = HashSet::new();
    let mut seen_captions = HashSet::new();
    let mut deduped = Vec::new();

    for image in images
        .into_iter()
        .filter(|image| usable_image_url(&image.url))
    {
        let url = image_dedupe_key(&image.url);
        if !seen_urls.insert(url) {
            continue;
        }
        // Only deduplicate by caption when it's non-empty
        let caption = caption_dedupe_key(&image.caption);
        if !caption.is_empty() && !seen_captions.insert(caption) {
            continue;
        }
        deduped.push(image);
    }

    deduped
}

fn extract_images_with_caption(
    root: &ElementRef<'_>,
    base: Option<&Url>,
    caption_override: Option<&str>,
    skip_images_in_figures: bool,
) -> Vec<CaptionedImage> {
    let pictures = root
        .select(&SEL_PICTURE)
        .filter(|picture| !skip_images_in_figures || !has_figure_ancestor(picture))
        .filter_map(|picture| {
            best_image_url(&picture, base).map(|url| CaptionedImage {
                url,
                caption: caption_override
                    .map(str::to_owned)
                    .or_else(|| first_img_alt(&picture))
                    .unwrap_or_default(),
            })
        })
        .collect::<Vec<_>>();

    if !pictures.is_empty() {
        return pictures;
    }

    root.select(&SEL_IMG)
        .filter(|img| !skip_images_in_figures || !has_figure_ancestor(img))
        .filter_map(|img| {
            best_image_url(&img, base).map(|url| CaptionedImage {
                url,
                caption: caption_override.map_or_else(|| img_alt(&img), str::to_owned),
            })
        })
        .collect()
}

fn best_image_url(root: &ElementRef<'_>, base: Option<&Url>) -> Option<String> {
    let mut candidates = Vec::new();
    let mut src_url: Option<String> = None;

    for source in root.select(&SEL_SOURCE) {
        collect_image_candidates(&source, base, &mut candidates);
    }
    for img in root.select(&SEL_IMG) {
        collect_image_candidates(&img, base, &mut candidates);
    }
    if root.value().name() == "img" || root.value().name() == "source" {
        // Collect src separately so we can prefer absolute src over relative srcset entries
        if let Some(src) = root.value().attr("src") {
            let url = resolve_url(base, src);
            if usable_image_url(&url) {
                src_url = Some(url);
            }
        }
        collect_image_candidates(root, base, &mut candidates);
    }

    // When src is an absolute URL, prefer it over candidates resolved from relative srcset paths
    // that produce a different domain
    if let Some(ref src) = src_url
        && let Some(src_domain) = Url::parse(src)
            .ok()
            .and_then(|u| u.host_str().map(String::from))
    {
        if let Some(best) = candidates
            .iter()
            .max_by(|a, b| image_candidate_score(a).total_cmp(&image_candidate_score(b)))
            && let Some(best_domain) = Url::parse(&best.url)
                .ok()
                .and_then(|u| u.host_str().map(String::from))
            && src_domain == best_domain
        {
            return Some(best.url.clone());
        }
        // src has a different domain from the best candidate — trust src
        return Some(src.clone());
    }

    candidates
        .into_iter()
        .max_by(|a, b| image_candidate_score(a).total_cmp(&image_candidate_score(b)))
        .map(|candidate| candidate.url)
}

fn collect_image_candidates(
    el: &ElementRef<'_>,
    base: Option<&Url>,
    candidates: &mut Vec<ImageCandidate>,
) {
    let fallback_width = el.value().attr("width").and_then(|w| w.parse::<f32>().ok());

    if let Some(srcset) = el.value().attr("srcset") {
        static RE_SRCSET: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(\S+)(?:\s+(\d+[wx]|[0-9.]+x))?\s*(?:,\s*|$)").unwrap());
        for cap in RE_SRCSET.captures_iter(srcset) {
            let src = cap.get(1).unwrap().as_str();
            let descriptor = cap.get(2).map(|m| m.as_str());
            let url = resolve_url(base, src);
            if usable_image_url(&url) {
                candidates.push(ImageCandidate {
                    url,
                    width: image_candidate_width(descriptor, fallback_width),
                });
            }
        }
    }

    if let Some(src) = el.value().attr("src") {
        let url = resolve_url(base, src);
        if usable_image_url(&url) {
            candidates.push(ImageCandidate {
                width: image_url_score(&url),
                url,
            });
        }
    }
}

fn image_candidate_score(candidate: &ImageCandidate) -> f32 {
    if candidate.width > 0.0 {
        candidate.width
    } else {
        image_url_score(&candidate.url)
    }
}

fn image_candidate_width(descriptor: Option<&str>, fallback_width: Option<f32>) -> f32 {
    descriptor
        .and_then(|d| {
            d.strip_suffix('w')
                .and_then(|width| width.parse::<f32>().ok())
                .or_else(|| {
                    d.strip_suffix('x')
                        .and_then(|scale| scale.parse::<f32>().ok())
                        .map(|scale| scale * fallback_width.unwrap_or(1000.0))
                })
        })
        .unwrap_or_default()
}

fn has_figure_ancestor(el: &ElementRef<'_>) -> bool {
    el.ancestors()
        .skip(1)
        .filter_map(ElementRef::wrap)
        .any(|ancestor| ancestor.value().name() == "figure")
}

fn figure_caption(figure: &ElementRef<'_>) -> Option<String> {
    figure
        .select(&SEL_FIGCAPTION_TEXT)
        .next()
        .or_else(|| figure.select(&SEL_FIGCAPTION).next())
        .map(clean_element_text)
        .filter(|caption| !caption.is_empty())
}

fn first_img_alt(root: &ElementRef<'_>) -> Option<String> {
    root.select(&SEL_IMG)
        .next()
        .map(|img| img_alt(&img))
        .filter(|alt| !alt.is_empty())
}

fn clean_element_text(el: ElementRef<'_>) -> String {
    clean_caption(&el.text().join(" "))
}

fn img_alt(img: &ElementRef<'_>) -> String {
    clean_caption(img.value().attr("alt").unwrap_or_default())
}

fn clean_caption(caption: &str) -> String {
    caption
        .split(" Credit:")
        .next()
        .unwrap_or(caption)
        .split_whitespace()
        .join(" ")
}

fn resolve_url(base: Option<&Url>, img_url: &str) -> String {
    let url = base
        .and_then(|b| b.join(img_url).ok())
        .map_or_else(|| img_url.to_string(), |u| u.to_string());
    normalize_image_url(&url)
}

fn usable_image_url(url: &str) -> bool {
    !url.is_empty()
        && !url.contains("placeholder")
        && !url.contains("gravatar.com")
        && !url.starts_with("data:")
}

fn image_url_score(url: &str) -> f32 {
    let width = infer_resize_width(url)
        .or_else(|| infer_path_width(url))
        .or_else(|| infer_largest_dimension_width(url))
        .unwrap_or_default();
    let lower_url = url.to_ascii_lowercase();
    let is_webp_extension = Url::parse(url)
        .ok()
        .and_then(|parsed| {
            Path::new(parsed.path())
                .extension()
                .map(|extension| extension.eq_ignore_ascii_case("webp"))
        })
        .unwrap_or(false);
    let format_bonus = if lower_url.contains("/format/webp") || is_webp_extension {
        0.0
    } else {
        0.1
    };
    width + format_bonus
}

fn image_dedupe_key(url: &str) -> String {
    let url = Url::parse(url)
        .ok()
        .and_then(|parsed| {
            // Try 'url' param first (common in CDN proxies), fall back to 'image'
            parsed.query_pairs().find_map(|(key, value)| {
                (key == "url" || key == "image").then(|| value.into_owned())
            })
        })
        .unwrap_or_else(|| url.to_string());

    Url::parse(&url).map_or(url, |mut parsed| {
        parsed.set_query(None);
        parsed.set_fragment(None);
        let domain = parsed.domain().unwrap_or_default().to_string();
        let path = parsed.path();
        if let Some(filename) = path.rsplit('/').next().and_then(canonical_image_filename)
            && filename.len() >= 12
        {
            return format!("{domain}/{filename}").to_ascii_lowercase();
        }

        let path = path
            .trim_start_matches('/')
            .split('/')
            .filter(|segment| {
                segment.parse::<u32>().is_err()
                    && !matches!(*segment, "format" | "resize" | "width" | "height")
            })
            .filter_map(canonical_image_filename)
            .join("/");
        format!("{domain}/{path}").to_ascii_lowercase()
    })
}

fn caption_dedupe_key(caption: &str) -> String {
    caption.split_whitespace().join(" ").to_ascii_lowercase()
}

fn canonical_image_filename(filename: &str) -> Option<String> {
    let mut filename = filename;
    while let Some((stem, extension)) = filename.rsplit_once('.') {
        if matches!(
            extension.to_ascii_lowercase().as_str(),
            "avif" | "gif" | "jpeg" | "jpg" | "png" | "webp"
        ) {
            filename = stem;
        } else {
            break;
        }
    }
    (!filename.is_empty()).then(|| filename.to_string())
}

fn infer_resize_width(url: &str) -> Option<f32> {
    RE_RESIZE_WIDTH
        .captures(url)?
        .get(1)?
        .as_str()
        .parse::<f32>()
        .ok()
}

fn infer_path_width(url: &str) -> Option<f32> {
    Url::parse(url)
        .ok()?
        .path_segments()?
        .find_map(|segment| segment.parse::<f32>().ok().filter(|width| *width >= 100.0))
}

fn infer_largest_dimension_width(url: &str) -> Option<f32> {
    RE_IMAGE_DIMENSIONS
        .captures_iter(url)
        .filter_map(|captures| {
            let width = captures.get(1)?.as_str().parse::<u32>().ok()?;
            let height = captures.get(2)?.as_str().parse::<u32>().ok()?;
            (width >= 100 && height >= 100).then_some(width)
        })
        .max()
        .map(|width| width as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use scraper::Html;

    #[test]
    fn figure_caption_takes_priority_over_image_alt() {
        let html = r#"
            <figure class="align-center zoomable">
                <a href="https://images.theconversation.com/files/740102/original/file-20260604-57-bv8jr5.jpg?ixlib=rb-4.1.0&q=45&auto=format&w=1000&fit=clip">
                    <div class="placeholder-container">
                        <img
                            alt="Two men seated in a wheelchair and on a mobile seated scooter wear blazers and smile at each other as their hands extend toward each other."
                            src="https://images.theconversation.com/files/740102/original/file-20260604-57-bv8jr5.jpg?ixlib=rb-4.1.0&q=45&auto=format&w=754&fit=clip"
                            srcset="https://images.theconversation.com/files/740102/original/file-20260604-57-bv8jr5.jpg?ixlib=rb-4.1.0&q=45&auto=format&w=600&h=400&fit=crop&dpr=1 600w, https://images.theconversation.com/files/740102/original/file-20260604-57-bv8jr5.jpg?ixlib=rb-4.1.0&q=30&auto=format&w=754&h=503&fit=crop&dpr=2 1508w"
                        >
                    </div>
                </a>
                <figcaption>
                    <span class="caption">Republican senators Jim Justice, left, of West Virginia and Mitch McConnell greet each other at the U.S. Capitol in Washington on June 1, 2026.</span>
                    <span class="attribution"><a class="source" href="https://www.gettyimages.com/detail/news-photo/senator-jim-justice-and-senator-mitch-mcconnell-high-five-news-photo/2278666035?adppopup=true">Nathan Posner/Anadolu via Getty Images</a></span>
                </figcaption>
            </figure>
        "#;
        let doc = Html::parse_fragment(html);
        let images = extract_images(&doc.root_element(), None);

        assert!(images.iter().any(|image| {
            image.caption
                == "Republican senators Jim Justice, left, of West Virginia and Mitch McConnell greet each other at the U.S. Capitol in Washington on June 1, 2026."
        }));
        assert!(
            !images
                .iter()
                .any(|image| { image.caption.starts_with("Two men seated in a wheelchair") })
        );
    }

    #[test]
    fn image_alt_is_used_when_figure_caption_is_missing() {
        let html = r#"
            <figure>
                <img alt="Fallback description" src="https://example.com/image-1200x800.jpg">
            </figure>
        "#;
        let doc = Html::parse_fragment(html);
        let images = extract_images(&doc.root_element(), None);

        assert!(
            images
                .iter()
                .any(|image| image.caption == "Fallback description")
        );
    }

    #[test]
    fn bbc_srcset_uses_largest_candidate_and_figcaption() {
        let html = r#"
            <figure data-travelling-actions-obscures="true" class="sc-70550624-0 bOanuU"><div data-testid="image" class="sc-79783b34-1 fMoqii"><img sizes="(min-width: 1280px) 50vw, (min-width: 1008px) 66vw, 96vw" srcset="https://ichef.bbci.co.uk/news/240/cpsprodpb/e765/live/324c5fe0-4d29-11f1-ac78-2112837ce2aa.jpg.webp 240w,https://ichef.bbci.co.uk/news/320/cpsprodpb/e765/live/324c5fe0-4d29-11f1-ac78-2112837ce2aa.jpg.webp 320w,https://ichef.bbci.co.uk/news/480/cpsprodpb/e765/live/324c5fe0-4d29-11f1-ac78-2112837ce2aa.jpg.webp 480w,https://ichef.bbci.co.uk/news/640/cpsprodpb/e765/live/324c5fe0-4d29-11f1-ac78-2112837ce2aa.jpg.webp 640w,https://ichef.bbci.co.uk/news/800/cpsprodpb/e765/live/324c5fe0-4d29-11f1-ac78-2112837ce2aa.jpg.webp 800w,https://ichef.bbci.co.uk/news/1024/cpsprodpb/e765/live/324c5fe0-4d29-11f1-ac78-2112837ce2aa.jpg.webp 1024w,https://ichef.bbci.co.uk/news/1536/cpsprodpb/e765/live/324c5fe0-4d29-11f1-ac78-2112837ce2aa.jpg.webp 1536w" src="https://ichef.bbci.co.uk/news/480/cpsprodpb/e765/live/324c5fe0-4d29-11f1-ac78-2112837ce2aa.jpg.webp" loading="lazy" alt="Nathalie Nahai Nathalie Nahai smiles at the camera" class="sc-79783b34-0 clLFUe"><span class="sc-79783b34-2 dUBtUF">Nathalie Nahai</span></div><figcaption class="sc-69115734-0 jsSfKy">Nathalie Nahai hopes that most people are cynical enough to see past a company mascot</figcaption></figure>
        "#;
        let doc = Html::parse_fragment(html);
        let images = extract_images(&doc.root_element(), None);

        assert!(images.iter().any(|image| {
            image.url == "https://ichef.bbci.co.uk/news/1536/cpsprodpb/e765/live/324c5fe0-4d29-11f1-ac78-2112837ce2aa.jpg.webp"
                && image.caption
                    == "Nathalie Nahai hopes that most people are cynical enough to see past a company mascot"
        }));
    }
}
