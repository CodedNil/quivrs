use super::*;
use crate::server::HTTP_CLIENT;
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
    content_not_contains: &'static [&'static str],
    images_contains: &'static [ExpectedImage],
}

#[derive(Clone, Copy)]
struct ExpectedImage {
    url: &'static str,
    caption: &'static str,
}

#[test]
#[ignore = "downloads HTML on first run, then uses /tmp/quivrs cache"]
fn bbc_article_full_parse() {
    assert_website_case(&WebsiteCase {
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
        content_not_contains: &[
            "Chris Marshall",
            "Documentaries",
            "Duolingo says its owl \"has become much more than a mascot\"",
            "Could humanoid robots be heading for the battlefield?",
            "Copyright 2026 BBC. All rights reserved.",
        ],
        images_contains: &[ExpectedImage {
            url: "https://ichef.bbci.co.uk/news/1536/cpsprodpb/e765/live/324c5fe0-4d29-11f1-ac78-2112837ce2aa.jpg.webp",
            caption: "Nathalie Nahai hopes that most people are cynical enough to see past a company mascot",
        }],
    });
}

#[test]
#[ignore = "downloads HTML on first run, then uses /tmp/quivrs cache"]
fn the_conversation_article_full_parse() {
    assert_website_case(&WebsiteCase {
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
        content_not_contains: &[
            "Become an author",
            "The authors do not work for, consult, own shares in or receive funding from any company or organization that would benefit from this article",
            "The shutdown – and the House’s inaction – helps pave Congress’ path to irrelevance",
            "Write an article and join a growing community of more than 227,300 academics and researchers from 5,537 institutions.",
        ],
        images_contains: &[ExpectedImage {
            url: "https://images.theconversation.com/files/740102/original/file-20260604-57-bv8jr5.jpg?auto=format&fit=clip&ixlib=rb-4.1.0&q=45&w=754",
            caption: "Republican senators Jim Justice, left, of West Virginia and Mitch McConnell greet each other at the U.S. Capitol in Washington on June 1, 2026.",
        }],
    });
}

#[test]
#[ignore = "downloads HTML on first run, then uses /tmp/quivrs cache"]
fn science_daily_article_full_parse() {
    assert_website_case(&WebsiteCase {
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
        content_not_contains: &[
            "RELATED STORIES",
            "This week, the world's governments are meeting in Geneva for the 78th meeting of the Standing Committee of CITES",
            "Stay informed with ScienceDaily's free email newsletter, updated daily and weekly",
            "Copyright 1995-2026 ScienceDaily or by other parties, where indicated",
            "Giant Fire Tornadoes Fight Oil Spills",
            "RELATED TOPICS",
            "Quirky",
        ],
        images_contains: &[ExpectedImage {
            url: "https://sciencedaily.com/images/1920/helmets-recovered-off-coast-of-benicarlo.webp",
            caption: "Helmets recovered off the coast of Benicarló were not Roman in origin, but formed part of a Late Medieval military cargo.",
        }],
    });
}

#[test]
#[ignore = "downloads HTML on first run, then uses /tmp/quivrs cache"]
fn proton_article_full_parse() {
    assert_website_case(&WebsiteCase {
        url: "https://proton.me/business/blog/europe-us-tech-dependence-qwant",
        domain: "proton.me",
        title: "Europe tech sovereignty is bigger than search defaults | Proton",
        summary: "European Parliament just dropped Google Search. It's a start. Here's why your business can't afford to wait for Europe's institutions to catch up.",
        published: "2026-06-05T14:48:01Z",
        tags: &[],
        content_contains: &[
            "By switching to Qwant, the EU is leading by example. Small adjustments can have a large impact",
            "The top three cloud providers — AWS, Microsoft Azure, and Google Cloud — are all American and together command an estimated 85% of the European cloud market",
            "How to start de-coupling from Big Tech",
            "Audit your stack. List every third-party tool your business uses and flag which are US-headquartered.",
        ],
        content_not_contains: &[
            "Large organizations aren’t paying ransomware threats anymore: SMBs are",
            "Secure your company’s emails, passwords, calendars, and more with Proton for Business.",
            "Partners and affiliates",
            "GDPR compliance",
            "We rebuilt Proton Drive’s engine. Now it’s up to 3x faster on all platforms.",
            "Ben is a writer and editor whose work has appeared in major newspapers",
            "Top 5 network security tools to protect your business",
        ],
        images_contains: &[ExpectedImage {
            url: "https://pmecdn.protonweb.com/image-transformation/?s=c&image=images/f_auto,q_auto/v1780663623/wp-pme/eu-tech-sovereignty-qwant-switch/eu-tech-sovereignty-qwant-switch.webp?_i=AA&width=1280&height=640",
            caption: "Image of a laptop with the EU flag and a lock on the screen with the US flag crossed out next to it",
        }],
    });
}

#[test]
#[ignore = "downloads HTML on first run, then uses /tmp/quivrs cache"]
fn twig_article_full_parse() {
    assert_website_case(&WebsiteCase {
        url: "https://thisweek.gnome.org/posts/2026/06/twig-252/",
        domain: "thisweek.gnome.org",
        title: "#252 Stronger Together",
        summary: "Updates on what happens across the GNOME project from week to week",
        published: "2026-06-06T00:00:00Z",
        tags: &[],
        content_contains: &[
            "A previewer companion for GNOME Files.",
            "You can test these changes in GNOME OS or by installing sushi and nautilus from the gnome-nightly Flatpak repository.",
            "Finally, in the diff view (the one you see on vertimus workflows) between a file and a previous version of a PO file",
        ],
        content_not_contains: &[
            "No matter if core, circle or third-party project - we are interested in all news",
        ],
        images_contains: &[
            ExpectedImage {
                url: "https://thisweek.gnome.org/_astro/image.DYzYXsTz_1DF9WT.webp",
                caption: "",
            },
            ExpectedImage {
                url: "https://thisweek.gnome.org/_astro/Vinyl.CrqFjvMH_Z1GeNC0.webp",
                caption: "",
            },
        ],
    });
}

#[test]
#[ignore = "downloads HTML on first run, then uses /tmp/quivrs cache"]
fn nasa_article_full_parse() {
    assert_website_case(&WebsiteCase {
        url: "https://www.nasa.gov/news-release/nasa-marches-toward-artemis-iii-mission-in-2027-names-crew-members/",
        domain: "nasa.gov",
        title: "NASA Marches Toward Artemis III Mission in 2027, Names Crew Members - NASA",
        summary: "Taking another step toward one of the most complex human spaceflight missions in recent history, NASA on Tuesday provided new Artemis III details and",
        published: "2026-06-09T16:12:02Z",
        tags: &["Artemis", "Artemis 3", "Missions"],
        content_contains: &[
            "During Artemis III, the agency’s SLS (Space Launch System) rocket will launch the Orion",
            "Europeans can take pride in being part of this exciting journey",
        ],
        content_not_contains: &["More NASA Social Accounts"],
        images_contains: &[ExpectedImage {
            url: "https://nasa.gov/wp-content/uploads/2026/06/artemis-iii-crew.jpg",
            caption: "The Artemis III crew poses for an official portrait (from left: Andre Douglas, Luca Parmitano, Randy Bresnik, Frank Rubio).",
        }],
    });
}

#[test]
#[ignore = "downloads HTML on first run, then uses /tmp/quivrs cache"]
fn openai_article_full_parse() {
    assert_website_case(&WebsiteCase {
        url: "https://openai.com/index/built-to-benefit-everyone-our-plan/",
        domain: "openai.com",
        title: "Built to benefit everyone: our plan",
        summary: "A vision for the future of AI, focusing on access, safety, and shared prosperity as OpenAI works to ensure AGI benefits everyone.",
        published: "2026-06-08T14:00:00Z",
        tags: &[],
        content_contains: &[
            "Imagine electricity reaching a rural American town in the 1920s. Before power lines arrived, daily life was shaped by physical limits: hauling water",
            "The first phase of OpenAI was about doing research toward AGI. The second phase began when our research became relevant",
        ],
        content_not_contains: &["Confidential submission of draft S-1 to the SEC"],
        images_contains: &[],
    });
}

#[test]
#[ignore = "downloads HTML on first run, then uses /tmp/quivrs cache"]
fn euronews_article_full_parse() {
    assert_website_case(&WebsiteCase {
        url: "https://www.euronews.com/my-europe/2026/06/09/bogs-almost-like-a-minefield-europes-wetlands-gain-military-importance",
        domain: "euronews.com",
        title: "Almost like minefields: Europe's peatlands gain military importance",
        summary: "Peatlands are more than carbon sinks and havens for rare species. Amid rising security tensions, they are seen across Europe as natural barriers to attackers.",
        published: "2026-06-09T07:44:22Z",
        tags: &[
            "Lithuania Brigade",
            "Lithuania",
            "Defence",
            "Germany",
            "nature",
            "nature conservation",
        ],
        content_contains: &[
            "The Baltic states – Estonia, Latvia and Lithuania – are examining, as part of the Baltic",
            "Even so, Jan Peters calls for careful language. Peatland restoration must not be understood as a way of sealing off Germany from EU and NATO partners",
        ],
        content_not_contains: &["Not investing in nature poses security risk"],
        images_contains: &[ExpectedImage {
            url: "https://images.euronews.com/articles/stories/09/78/37/54/1920x1080_cmsv2_ef53f4f3-ff3e-5f6a-a375-8f776543ce97-9783754.jpg",
            caption: "Peatlands cover only about three per cent of the world’s land surface, yet store roughly twice as much carbon as the biomass of all the planet’s forests.",
        }],
    });
}

fn assert_website_case(case: &WebsiteCase) {
    let _ = dioxus::logger::init(tracing::Level::INFO);

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

    // Verify images are real by checking HTTP response (in parallel)
    let urls: Vec<String> = article.images.iter().map(|img| img.url.clone()).collect();
    let results = runtime.block_on(async {
        futures::future::join_all(urls.iter().map(|url| async move {
            let resp = HTTP_CLIENT.head(url).send().await.unwrap();
            let status = resp.status();
            (url.clone(), status)
        }))
        .await
    });
    for (url, status) in &results {
        assert!(status.is_success(), "image URL returns {status}: {url}");
    }

    for expected in case.content_contains {
        assert!(
            article.content.contains(expected),
            "content did not contain {expected:?}"
        );
    }

    for expected in case.content_not_contains {
        assert!(
            !article.content.contains(expected),
            "content contained {expected:?}"
        );
    }

    for expected in case.images_contains {
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
