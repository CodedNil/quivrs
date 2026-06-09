use super::*;
use chrono::{DateTime, Utc};

#[derive(Clone, Copy)]
struct WebsiteCase {
    url: &'static str,
    domain: &'static str,
    title: &'static str,
    summary: &'static str,
    published: &'static str,
    tags: &'static [&'static str],
    content_contains: &'static [&'static str],
    images: &'static [ExpectedImage],
}

#[derive(Clone, Copy)]
struct ExpectedImage {
    url: &'static str,
    caption: &'static str,
}

macro_rules! website_case {
    ($name:ident => $case:expr) => {
        #[test]
        #[ignore = "downloads HTML on first run, then uses /tmp/quivrs cache"]
        fn $name() {
            assert_website_case(&$case);
        }
    };
}

const BBC_ARTICLE_FULL_PARSE: WebsiteCase = WebsiteCase {
    url: "https://www.bbc.com/news/articles/c99l1zzp8xzo",
    domain: "bbc.com",
    title: "Big tech bets on new mascots in bid to seem more cuddly",
    summary: "The likes of Apple, Microsoft and Google are all putting cartoon characters centre stage.",
    published: "2026-05-13T23:08:37.595Z",
    tags: &["News", "Business"],
    content_contains: &[
        "Tech giants Microsoft and Apple",
        "Nathalie Nahai",
        "Duolingo",
    ],
    images: &[ExpectedImage {
        url: "https://ichef.bbci.co.uk/news/1536/cpsprodpb/e765/live/324c5fe0-4d29-11f1-ac78-2112837ce2aa.jpg.webp",
        caption: "Nathalie Nahai hopes that most people are cynical enough to see past a company mascot",
    }],
};
website_case! { bbc_article_full_parse => BBC_ARTICLE_FULL_PARSE }

const THE_CONVERSATION_ARTICLE_FULL_PARSE: WebsiteCase = WebsiteCase {
    url: "https://theconversation.com/both-democrats-and-republicans-give-millions-to-universities-in-earmarks-but-not-in-the-same-way-281721",
    domain: "theconversation.com",
    title: "Both Democrats and Republicans give millions to universities in earmarks – but not in the same way",
    summary: "Democrats tend to give less than Republicans when it comes to earmarked funding for universities – but they give more to minority-serving institutions.",
    published: "2026-06-08T12:33:08Z",
    tags: &[
        "Public Universities",
        "US Congress",
        "US higher education",
        "US politics",
        "earmarks",
    ],
    content_contains: &[
        "U.S. politicians have",
        "Federal earmarks",
        "historically Black colleges and universities",
    ],
    images: &[ExpectedImage {
        url: "https://images.theconversation.com/files/740102/original/file-20260604-57-bv8jr5.jpg?auto=format&fit=clip&ixlib=rb-4.1.0&q=45&w=754",
        caption: "Republican senators Jim Justice, left, of West Virginia and Mitch McConnell greet each other at the U.S. Capitol in Washington on June 1, 2026.",
    }],
};
website_case! { the_conversation_article_full_parse => THE_CONVERSATION_ARTICLE_FULL_PARSE }

const SCIENCE_DAILY_ARTICLE_FULL_PARSE: WebsiteCase = WebsiteCase {
    url: "https://sciencedaily.com/releases/2026/06/260606075515.htm",
    domain: "sciencedaily.com",
    title: "Everyone thought these helmets were Roman until scientists uncovered the truth",
    summary: "Researchers have solved a decades-old mystery by showing that a cache of 43 helmets found off the Spanish coast is medieval, not Roman. The remarkable discovery exposes a thriving weapons trade network that connected Mediterranean powers during a time of piracy, warfare, and growing demand for military equipment.",
    published: "2026-06-08T11:39:10Z",
    tags: &[
        "Ancient Civilizations",
        "Lost Treasures",
        "Origin of Life",
        "Fossils",
        "Conflict",
        "Travel and Recreation",
        "Public Health",
        "Educational Policy",
    ],
    content_contains: &[
        "The study, led by researchers at the University of Alicante (UA)",
        "One of the study's most important advances was the use of an analytical method",
        "Piracy, Warfare, and Demand for Weapons",
    ],
    images: &[ExpectedImage {
        url: "https://sciencedaily.com/images/1920/helmets-recovered-off-coast-of-benicarlo.webp",
        caption: "Helmets recovered off the coast of Benicarló were not Roman in origin, but formed part of a Late Medieval military cargo.",
    }],
};
website_case! { science_daily_article_full_parse => SCIENCE_DAILY_ARTICLE_FULL_PARSE }

const PROTON_ARTICLE_FULL_PARSE: WebsiteCase = WebsiteCase {
    url: "https://proton.me/business/blog/europe-us-tech-dependence-qwant",
    domain: "proton.me",
    title: "Europe tech sovereignty is bigger than search defaults | Proton",
    summary: "European Parliament just dropped Google Search. It's a start. Here's why your business can't afford to wait for Europe's institutions to catch up.",
    published: "2026-06-05",
    tags: &[],
    content_contains: &[
        "By switching to Qwant, the EU is leading by example. Small adjustments can have a large impact",
        "The top three cloud providers — AWS, Microsoft Azure, and Google Cloud — are all American and together command an estimated 85% of the European cloud market",
        "How to start de-coupling from Big Tech",
        "Audit your stack. List every third-party tool your business uses and flag which are US-headquartered.",
    ],
    images: &[ExpectedImage {
        url: "https://pmecdn.protonweb.com/image-transformation/?s=c&image=images/f_auto,q_auto/v1780663623/wp-pme/eu-tech-sovereignty-qwant-switch/eu-tech-sovereignty-qwant-switch.webp?_i=AA&width=1280&height=640",
        caption: "Helmets recovered off the coast of Benicarló were not Roman in origin, but formed part of a Late Medieval military cargo.",
    }],
};
website_case! { proton_article_full_parse => PROTON_ARTICLE_FULL_PARSE }

fn assert_website_case(case: &WebsiteCase) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let article = runtime
        .block_on(fetch_source_content(case.url, None))
        .unwrap()
        .unwrap();

    assert_eq!(article.domain, case.domain);
    println!("Domain: {}", article.domain);
    assert_eq!(article.title, case.title);
    println!("Title: {}", article.title);
    assert_eq!(article.summary, case.summary);
    println!("Summary: {}", article.summary);
    assert_eq!(article.tags, case.tags);
    println!("Tags: {:?}", article.tags);
    assert_eq!(
        article.published,
        DateTime::parse_from_rfc3339(case.published)
            .unwrap()
            .with_timezone(&Utc)
    );
    println!("Date: {}", article.published);

    for expected in case.content_contains {
        assert!(
            article.content.contains(expected),
            "content did not contain {expected:?}"
        );
    }

    for expected in case.images {
        assert!(
            article
                .images
                .iter()
                .any(|image| image.url == expected.url && image.caption == expected.caption),
            "expected image {:#?}; got {:#?}",
            (expected.url, expected.caption),
            article
                .images
                .iter()
                .map(|image| (&image.url, &image.caption))
                .collect::<Vec<_>>()
        );
    }
    println!("Images: {:?}", article.images);
}
