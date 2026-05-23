use crate::{
    server::database::get_rated_article_embeddings,
    shared::{ArticleSource, Category, Rating},
};
use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use itertools::Itertools;
use sha2::{Digest, Sha256};
use std::{fmt::Write, sync::LazyLock};
use strum::IntoEnumIterator;
use tokio::sync::{Mutex, OnceCell};

// --- Label writing guide ---
//
// Labels are embedded with EmbeddingGemma 300M and classified via cosine similarity.
// The label embedding is a mean-pool over all its token embeddings, so every token
// contributes equally. Write labels as dense keyword lists, not prose.
//
// Rules:
//   - Use single distinctive tokens. "jailed" beats "sentenced to prison".
//   - Prefer named things: brands, people, places, organisations (NHS, Oscars, Starmer).
//     They are uniquely associated with one category and carry strong signal.
//   - Avoid tokens that appear across multiple labels — they dilute discrimination.
//   - No negations: "not marketing" adds "marketing" semantics to the vector.
//   - No filler prose: "Includes coverage of" wastes tokens on noise.
//   - Multi-word phrases are fine when the compound is widely known (Premier League,
//     Black Friday), but single tokens are preferred where one exists.
//   - Commas make no difference to the embedding — they are just punctuation tokens.
// ---

const MODEL: EmbeddingModel = EmbeddingModel::EmbeddingGemma300M;
pub const MODEL_NAME: &str = "EmbeddingGemma300M";

pub const fn category_label(category: Category) -> &'static str {
    match category {
        // Corporate finance, stock markets, and macroeconomics
        Category::Business => {
            "earnings revenue profit stocks shares merger acquisition IPO dividends CEO layoffs tariffs GDP inflation hedge-fund private-equity startup venture-capital fiscal bonds FTSE Nasdaq shareholder buyout trade-deal commerce markets recession bookkeeping audit liquidation bankruptcy conglomerate arbitrage commodity-trading insolvency mercantile quarterly-results acquisition-bid hostile-takeover manufacturing supply-chain logistics factory automation textiles outsourcing"
        }
        // Government, parliament, elections, political parties, and military/defence affairs
        Category::Politics => {
            "parliament MPs chancellor legislation elections Conservative Labour LibDems Reform cabinet minister Starmer Farage Reeves Sunak Trump Putin vote constituency manifesto polling Holyrood Whitehall PMQs SNP military army navy RAF defence war NATO troops Israel missile drone Treasury Downing-Street backbencher frontbench state-visit diplomacy geopolitics referendum electorate bicameral devolution incumbency Democrats Republicans Libertarians Mayor Congress Senate White-House Pentagon Kremlin Bundestag Elysée sanctions embassy treaty bipartisan gubernatorial impeachment autocracy federalism by-election turnout benefits welfare social-security DWP councillor"
        }
        // Courts, crime, police investigations, and criminal justice
        Category::Law => {
            "court trial convicted jailed sentenced defendant prosecution verdict jury judge murder stabbing knife beating assault attack grooming exploitation abuse offender indecent coercive stalking fraud theft robbery ASA FCA banned misconduct DWP lawsuit acquitted injunction plaintiff caution probe tribunal litigation legal-action manslaughter burglary shoplifting extradition forensics bailiff subpoena affidavit perjury jurisprudence embezzlement investigation missing-person inquest coroner detective tragedy drowned rescuers tribute police-cordon rape custodial-sentence anonymity-order County-Court FTC"
        }
        // Clinical medicine and personal health — NHS, diagnosis, treatment, fitness, diet
        Category::Health => {
            "NHS hospital GP vaccine diagnosis cancer surgery prescription clinical trial symptoms disease epidemic inpatient psychiatric pharmaceutical medication therapy chronic disability overdose ward nurse stroke dementia autism mental health fitness diet nutrition exercise workout weight sleep supplements vitamins cardiology oncology immunology epidemiology neurology pathology radiology physiotherapy emergency-services first-responders paramedics palliative hospice melanoma"
        }
        // Entertainment, film, TV, music, celebrities, streaming, and media
        Category::Culture => {
            "Netflix Disney streaming celebrities actor director Oscars BAFTA album concert band horror thriller documentary royals King Queen Royal entertainment Grammy Glastonbury blockbuster sitcom season episode premiere trailer how to watch film broadcaster Ofcom Channel 4 BBC media paparazzi showbiz cinematography choreography discography playwright red-carpet stardom influencer Hollywood Bollywood Cannes Sundance Eurovision K-pop Broadway West-End exhibition gallery museum sculpture literature bestseller casting audition cheese-rolling YouTubers festival headlining gnomes chart-topper rapper studio-album ITV Bafta novel author book"
        }
        // Domestic life, cooking, home, fashion, consumer tips, and personal finance
        Category::Lifestyle => {
            "recipe cooking cleaning decor fashion wardrobe travel garden kitchen wedding household mattress bedding interior skincare bathroom laundry storage hacks tips ants vinegar coffee mortgage rent savings pension budget bills energy loan credit housing property landlord employment job unemployed graduate parenting housekeeping upholstery sourdough minimalism staycation decluttering novel author island-life wardens remote-living dating"
        }
        // Cars, transport infrastructure, aviation, and commuting
        Category::Transport => {
            "car EV electric vehicle Tesla road motorway highway aviation airline airport train railway commute driver fuel petrol diesel autonomous self-driving bus lorry van cycling infrastructure logistics freight aeronautics maritime-shipping locomotive fuselage tarmac air-traffic-control haulage tramway Port-of-Dover"
        }
        // Wildlife, ecology, the natural world, and climate — animals, plants, weather, conservation
        Category::Nature => {
            "wildlife animals birds insects plants trees forests oceans seas rivers mountains countryside national parks ecology biodiversity extinction endangered conservation habitat hedgehog deer butterfly orchid climate carbon flooding drought heatwave rewilding farming weather meteorology zoology botany entomology ornithology ecosystem reforestation precipitation humidity anticyclone pesticides waste-sites tonnes-of-waste spider insect fish butterfly shark elephant dog cat rabbit hamster"
        }
        // Consumer electronics and hardware — phones, laptops, TVs, headphones, wearables
        Category::Technology => {
            "smartphone iPhone Android laptop notebook tablet smartwatch wearable headphones on-ear over-ear earphones audio speakers display OLED television TV camera drone VPN router hardware gadget Apple Samsung LG Sony Google Meta Facebook Twitter Microsoft Huawei Lenovo IBM Oracle Nvidia AMD console GPU peripherals motherboard chipset bluetooth firmware-update pixel-density megapixel print-methods"
        }
        // Software development, coding, and cybersecurity
        Category::Software => {
            "coding programming Python Ruby Rust JavaScript TypeScript HTML CSS web websites React Node GitHub API open-source Linux algorithm compiler binary firmware SDK cybersecurity breach hacking phishing malware ransomware exploit vulnerability CVE backend frontend devops refactoring middleware multithreading encryption-key repository containerization"
        }
        // Artificial intelligence, machine learning, and AI assistants
        Category::AI => {
            "AI ChatGPT Claude Gemini GPT OpenAI Anthropic DeepMind machine-learning LLM neural-network model-training inference chatbot Copilot Midjourney Stable-Diffusion robotics generative-AI singularity transformer-architecture backpropagation reinforcement-learning weights-and-biases fine-tuning hallucinations prompt-engineering superintelligence web-scraping crawlers data-mining synthetic-data"
        }
        // Scientific research, astronomy, biology, and academic discovery
        Category::Science => {
            "astronomy telescope genome species paleontology neuroscience botany physics quantum biology chemistry fossils stargazing celestial earthshine mathematics NASA rocket satellite spacecraft orbit Mars experiment journal hypothesis lab discovery space-exploration astrophysics particle-accelerator CRISPR relativity thermodynamics microbiology molecular-genetics exoplanet super-collider vaccine-trials"
        }
        // Physical competitive sports and athletic leagues
        Category::Sports => {
            "football cricket tennis rugby golf boxing swimming cycling wicket Premier-League Champions-League Europa-League Bundesliga La-Liga NBA NFL MLB transfer-window squad batting Wimbledon Olympics F1 Grand-Prix athletics marathon heptathlon scoreboard goalscorer touchdown wicket-keeper grand-slam tie-break velodrome podium-finish paralympics World-Cup trophy national-team call-up"
        }
        // Video games, gaming culture, esports, and game releases
        Category::Gaming => {
            "game gameplay esports Fortnite Minecraft Steam PlayStation Xbox Nintendo Switch RPG FPS shooter strategy multiplayer co-op indie AAA Zelda Mario studio console handheld retrogaming speedrun microtransactions loot-box ray-tracing frame-rate walkthrough boss-fight"
        }
    }
}

pub fn label_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hasher.update(b"\0");
    hasher.update(MODEL_NAME.as_bytes());
    hasher.finalize().iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

static EMBEDDING_MODEL: LazyLock<Mutex<TextEmbedding>> = LazyLock::new(|| {
    Mutex::new(
        TextEmbedding::try_new(InitOptions::new(MODEL).with_show_download_progress(true))
            .expect("Failed to load embedding model"),
    )
});

async fn generate_embeddings(texts: &[String]) -> Result<Vec<Vec<f32>>> {
    EMBEDDING_MODEL.lock().await.embed(texts, None)
}

pub fn article_text(s: &ArticleSource, title_repeat: usize) -> String {
    std::iter::repeat_n(s.title.replace('\n', " "), title_repeat)
        .chain([
            s.tags.iter().take(8).join(", ").replace('\n', " "),
            s.summary
                .chars()
                .take(200)
                .collect::<String>()
                .replace('\n', " "),
        ])
        .filter(|field| !field.is_empty())
        .join(". ")
}

pub async fn generate_article_embeddings(articles: &[ArticleSource]) -> Result<Vec<Vec<f32>>> {
    let texts: Vec<String> = articles.iter().map(|a| article_text(a, 5)).collect();
    let mut embs = generate_embeddings(&texts).await?;
    for emb in &mut embs {
        normalize(emb);
    }
    Ok(embs)
}

pub async fn embed_label_texts(texts: &[String]) -> Result<Vec<Vec<f32>>> {
    let mut embs = generate_embeddings(texts).await?;
    for emb in &mut embs {
        normalize(emb);
    }
    Ok(embs)
}

fn normalize(v: &mut Vec<f32>) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v {
            *x /= norm;
        }
    }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

static LABEL_EMBEDDINGS: OnceCell<Vec<Vec<f32>>> = OnceCell::const_new();

pub async fn seed_label_cache(cat: Vec<Vec<f32>>) {
    let _ = LABEL_EMBEDDINGS.set(cat);
}

pub async fn classify(embedding: &[f32]) -> Result<Category> {
    let labels = LABEL_EMBEDDINGS
        .get_or_try_init(|| async {
            let texts: Vec<String> = Category::iter()
                .map(|c| format!("title: none | text: {}", category_label(c)))
                .collect();
            anyhow::Ok(embed_label_texts(&texts).await?)
        })
        .await?;

    let best_cat = Category::iter()
        .zip(labels)
        .max_by(|(_, a), (_, b)| {
            cosine_similarity(embedding, a).total_cmp(&cosine_similarity(embedding, b))
        })
        .map(|(v, _)| v)
        .unwrap();

    Ok(best_cat)
}

pub async fn estimate_liked(embedding: &[f32]) -> Result<f32> {
    let rated = get_rated_article_embeddings().await?;
    if rated.is_empty() {
        return Ok(0.5);
    }

    let mut total_weight = 0.0;
    let mut weighted_sum = 0.0;

    for (rating, rated_emb) in rated {
        let sim = cosine_similarity(embedding, &rated_emb);
        if sim > 0.65 {
            let weight = (sim * 12.0).exp();
            let score_value = match rating {
                Rating::Loved => 1.0,
                Rating::Liked => 0.75,
                Rating::Neutral => 0.5,
                Rating::Disliked => 0.25,
                Rating::Hated => 0.0,
            };

            weighted_sum += score_value * weight;
            total_weight += weight;
        }
    }

    if total_weight == 0.0 {
        // If no articles are similar enough, we return a neutral 0.5
        return Ok(0.5);
    }

    Ok(weighted_sum / total_weight)
}
