use crate::{
    server::database,
    shared::{Category, PendingSource, Rating},
};
use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use itertools::Itertools;
use sha2::{Digest, Sha256};
use std::{fmt::Write, sync::LazyLock};
use strum::IntoEnumIterator;
use tokio::sync::Mutex;

const MODEL: EmbeddingModel = EmbeddingModel::EmbeddingGemma300M;
pub const EMBEDDING_MODEL_NAME: &str = "EmbeddingGemma300M";

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

const fn category_label(category: Category) -> &'static [&'static str] {
    match category {
        // Corporate finance, stock markets, and macroeconomics
        Category::Business => &[
            "Corporate finance, business earnings reports, revenue, profits, stocks, shares, and market trading.",
            "Company mergers, acquisitions, takeovers, IPOs, corporate restructuring, and CEO layoffs.",
            "Macroeconomics, GDP data, inflation rates, interest rates, central bank tariffs, and national recessions.",
            "Venture capital, startup investments, private equity, hedge funds, and market capitalization.",
        ],
        // Government, parliament, elections, political parties, and military/defence affairs
        Category::Politics => &[
            "Government politics, parliament debates, legislation, MPs, ministers, policies, and Whitehall decisions.",
            "General elections, political party campaigns, voting polls, manifestos, by-elections, and electorate dynamics.",
            "Military affairs, defense spending, army, navy, RAF, NATO, troop movements, and global warfare.",
            "Geopolitics, international diplomacy, state visits, foreign policy, and bilateral trade agreements.",
            "UK domestic political figures and commentary involving Starmer, Sunak, Farage, Reeves, or Downing Street.",
        ],
        // Courts, crime, police investigations, and criminal justice
        Category::Law => &[
            "Court trials, criminal convictions, judges, juries, defense lawyers, prosecution, and legal verdicts.",
            "Police investigations, criminal justice, arrests, forensics, raids, and law enforcement probes.",
            "Violent crimes, murder, assault, stabbing, theft, robbery, fraud, embezzlement, and legal misconduct.",
            "Lawsuits, high court injunctions, litigation, tribunals, regulatory bans, and civil legal action.",
        ],
        // Clinical medicine and personal health — NHS, diagnosis, treatment, fitness, diet
        Category::Health => &[
            "Clinical medicine, hospital operations, surgeries, emergency departments, wards, and NHS healthcare systems.",
            "Medical diagnoses, cancer treatments, chronic diseases, prescription medications, pharmaceuticals, and symptoms.",
            "Public health, virus epidemics, pathogen outbreaks, vaccinations, and epidemiology tracking.",
            "Personal fitness, nutrition, diet plans, workout routines, mental health therapy, and wellness.",
        ],
        // Entertainment, film, TV, music, celebrities, streaming, and media
        Category::Culture => &[
            "Entertainment media, streaming shows, TV series episodes, seasonal premieres, trailers, and broadcasters.",
            "Celebrity culture, pop stars, actors, red carpet events, paparazzi, influencers, and Hollywood showbiz.",
            "Music albums, concerts, bands, music festivals, Glastonbury, and industry awards like the Grammys or BAFTAs.",
            "Fine arts, theatre productions, literature, novel authors, museum exhibitions, and cultural reviews.",
        ],
        // Domestic life, cooking, home, fashion, consumer tips, and personal finance
        Category::Lifestyle => &[
            "Home cooking, baking recipes, meal prep, culinary tips, and kitchen design.",
            "Interior decor, cleaning hacks, household management, gardening, and property maintenance.",
            "Personal finance, mortgages, rent, household energy bills, savings, pensions, and budgeting advice.",
            "Fashion trends, wardrobe styles, skincare routines, dating life, parenting, and travel staycations.",
        ],
        // Cars, transport infrastructure, aviation, and commuting
        Category::Transport => &[
            "Electric vehicles, EVs, consumer cars, automotive engineering, roads, and motorway infrastructure.",
            "Aviation industry, airlines, airport operations, commercial flights, and airspace management.",
            "Rail networks, commuter trains, subways, public transit systems, and cargo freight shipping.",
        ],
        // Wildlife, ecology, the natural world, and climate — animals, plants, weather, conservation
        Category::Nature => &[
            "Wildlife conservation, natural ecosystems, biodiversity, endangered animal species, and birds.",
            "Climate change, global warming, carbon emissions, extreme weather events, floods, and droughts.",
            "Environmental science, forestry, meteorology, countryside ecology, and marine biology.",
        ],
        // Consumer electronics and hardware — phones, laptops, TVs, headphones, wearables
        Category::Technology => &[
            "Consumer electronics, smartphones, laptops, hardware components, and gadget specifications.",
            "Computer hardware, microchips, GPUs, motherboards, solid-state drives, and display panels.",
            "Smart home automation, wearable devices, audio speakers, and wireless routing peripherals.",
        ],
        // Software development, coding, and cybersecurity
        Category::Software => &[
            "Software development, coding practices, programming languages like Python, Rust, or JavaScript.",
            "Cybersecurity breaches, hacking incidents, malware exploits, ransomware, and system vulnerabilities.",
            "DevOps infrastructure, database architecture, open-source repositories, APIs, and cloud services.",
        ],
        // Artificial intelligence, machine learning, and AI assistants
        Category::AI => &[
            "Artificial intelligence, machine learning, large language models, LLMs, and neural networks.",
            "Generative AI tools, chatbots like ChatGPT or Claude, prompt engineering, and image synthesis.",
            "AI safety research, model training, parameter counts, and autonomous agent development.",
        ],
        // Scientific research, astronomy, biology, and academic discovery
        Category::Science => &[
            "Scientific research papers, laboratory experiments, breakthroughs, hypotheses, and academic journals.",
            "Space exploration, astronomy, telescopes, NASA rocket launches, satellites, and planetary discoveries.",
            "Theoretical physics, quantum mechanics, chemistry equations, biology, genetics, and molecular research.",
        ],
        // Physical competitive sports and athletic leagues
        Category::Sports => &[
            "Football match reports, Premier League fixtures, Champions League results, goals, and team lineups.",
            "Competitive athletic sports, cricket wickets, tennis grand slams, rugby scrums, and golf tournaments.",
            "Sports punditry, manager selections, team of the season reviews, transfer window signings, and squads.",
            "The Olympic games, professional athletes, marathons, motorsports F1 Grand Prix, and podium finishes.",
        ],
        // Video games, gaming culture, esports, and game releases
        Category::Gaming => &[
            "Video game releases, gameplay mechanics, console hardware, and esports tournaments.",
            "Gaming culture, streaming channels, online multiplayer matches, and indie studio developments.",
        ],
    }
}

pub fn sentiment_label(sentiment: &str) -> &'static str {
    match sentiment {
        // Uplifting, solutions-oriented, or curiosity-inducing framing
        "positive" => {
            "positive milestone breakthrough triumph progress success solution forward-looking heartwarming unity inspiring resilience recovery champion recovery masterpiece miracle innovation discovery genius curiosity fascinating mystery secret unexpected intriguing awe-inspiring milestone unique pioneer brilliant forward-step boost stellar incredible superb outstanding together community harmony optimism"
        }
        // Standard, dry, inverted-pyramid style reporting
        "neutral" => {
            "neutral reported stated announced said according scheduled timing figures updates minutes sequential record timeline log confirmed data source statistics summary documentation noted stated index routine outline list briefing filed ongoing process regular overview standard history background details chronologically itemized"
        }
        // Cynical, critical, aggressive, or tragic framing
        "negative" => {
            "negative toxic outrage controversy backlash condemnation failure disaster scandal fury brutal devastating threat bleak warning critical crisis tragedy investigation slammed gridlock error lawsuit dispute worst-case hostile bleak chaos bitter panic blame ruined fault shocking horrific collapse dangerous failure bleeding nightmare warning"
        }
        _ => "",
    }
}

pub fn importance_label(importance: &str) -> &'static str {
    match importance {
        // Macro-scale structural changes, high-consequence policy, and permanence
        "important" => {
            "important historic monumental unprecedented permanent landmark fundamental crisis systemic global national macro widespread structural existential core priority turning-point paradigm-shift definitive far-reaching foundation critical essential massive fundamental long-term key-factor major-overhaul catalyst prime strategic sweeping major dominant core emergency tectonic-shift paramount"
        }
        // Regular everyday news, localized beats, and expected standard reports
        "neutral" => {
            "neutral routine standard localized regular baseline typical commonplace regional current-events standard-reporting ongoing intermediate moderate current seasonal updating periodic standard-issue expected ordinary normal mid-tier secondary regional-news day-to-day typical-case local-market update-on status-quo tracking standard-procedure follow-up ongoing-investigation"
        }
        // Ephemeral, fleeting, entertainment gossip, or hyper-local novelties
        "trivial" => {
            "trivial viral-video social-media celebrity fan-reaction trending spotting instagram tik-tok post photo meme watch-the-moment reaction user-comments wardrobe outfit spotted internet-divided split-opinion sidebar quiz challenge lifestyle-hack unboxing spoiler-alert red-carpet outfit dating-rumor prank fast-fashion horoscope recipe clip-goes-viral joke gossip funny-video"
        }
        _ => "",
    }
}

pub fn label_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hasher.update(b"\0");
    hasher.update(EMBEDDING_MODEL_NAME.as_bytes());
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

pub fn article_text(s: &PendingSource, title_repeat: usize) -> String {
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

pub async fn generate_article_embeddings(texts: &[String]) -> Result<Vec<Vec<f32>>> {
    let mut embs = generate_embeddings(texts).await?;
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

pub async fn classify(article_embedding: &[f32]) -> Result<Category> {
    let mut classification_targets = Vec::new();

    for cat in Category::iter() {
        let sub_labels = category_label(cat);
        for (idx, sub_label_text) in sub_labels.iter().enumerate() {
            classification_targets.push((
                cat,
                format!("cat_{cat}_sub_{idx}"),
                label_hash(sub_label_text),
                (*sub_label_text).to_string(),
            ));
        }
    }

    let db_payload: Vec<_> = classification_targets
        .iter()
        .map(|(_, unique_id, hash, text)| (unique_id.clone(), hash.clone(), text.clone()))
        .collect();

    let embedded_vectors = database::get_or_refresh_label_embeddings_batch(&db_payload).await?;

    let mut best_category = Category::Politics;
    let mut highest_similarity = -1.0f32;

    for ((category, _, _, _), label_vector) in classification_targets.iter().zip(embedded_vectors) {
        let similarity = cosine_similarity(article_embedding, &label_vector);
        if similarity > highest_similarity {
            highest_similarity = similarity;
            best_category = *category;
        }
    }

    Ok(best_category)
}

pub fn calculate_preference_score(embedding: &[f32], rated: &[(Rating, Vec<f32>)]) -> f64 {
    if rated.is_empty() {
        return 0.5;
    }

    let (sum, weight) = rated
        .iter()
        .map(|(rating, rated_emb)| {
            let sim = cosine_similarity(embedding, rated_emb);
            (rating, sim)
        })
        .fold((0.0, 0.0), |(s, w), (rating, sim)| {
            // We use a steep exponential power (sim^8 or exp(sim * 10))
            // so that high similarity has much more weight than low similarity,
            // but even low similarity still provides a "hint".
            let p = (sim * 10.0).exp();
            let val = match rating {
                Rating::Loved => 1.0,
                Rating::Liked => 0.75,
                Rating::Neutral => 0.5,
                Rating::Disliked => 0.25,
                Rating::Hated => 0.0,
            };
            (s + val * p, w + p)
        });

    if weight > 0.0 {
        f64::from(sum / weight)
    } else {
        0.5
    }
}

pub async fn get_sentiment_score(embedding: &[f32]) -> f64 {
    let keys = ["positive", "negative", "neutral"];
    let entries: Vec<_> = keys
        .iter()
        .map(|&k| {
            let text = sentiment_label(k);
            (format!("sentiment_{k}"), label_hash(text), text.to_string())
        })
        .collect();

    let embs = database::get_or_refresh_label_embeddings_batch(&entries)
        .await
        .unwrap_or_default();
    let weights: Vec<f32> = embs
        .iter()
        .map(|e| (cosine_similarity(embedding, e) * 10.0).exp())
        .collect();
    let sum: f32 = weights.iter().sum();

    // Score: Positive (1.0), Neutral (0.5), Negative (0.0)
    f64::from(weights[0].mul_add(1.0, weights[2] * 0.5) / sum)
}

pub async fn get_importance_score(embedding: &[f32]) -> f64 {
    let keys = ["important", "neutral", "trivial"];
    let entries: Vec<_> = keys
        .iter()
        .map(|&k| {
            let text = importance_label(k);
            (
                format!("importance_{k}"),
                label_hash(text),
                text.to_string(),
            )
        })
        .collect();

    let embs = database::get_or_refresh_label_embeddings_batch(&entries)
        .await
        .unwrap_or_default();
    let weights: Vec<f32> = embs
        .iter()
        .map(|e| (cosine_similarity(embedding, e) * 10.0).exp())
        .collect();
    let sum: f32 = weights.iter().sum();

    // Score: Important (1.0), Neutral (0.5), Trivial (0.0)
    f64::from(weights[0].mul_add(1.0, weights[2] * 0.5) / sum)
}
