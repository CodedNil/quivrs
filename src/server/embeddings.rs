use crate::shared::{Category, PendingSource, Rating};
use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use itertools::Itertools;
use sha2::{Digest, Sha256};
use std::{fmt::Write, sync::LazyLock};
use strum::IntoEnumIterator;
use tokio::sync::{Mutex, OnceCell};

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

pub const fn category_label(category: Category) -> &'static str {
    match category {
        // Corporate finance, stock markets, and macroeconomics
        Category::Business => {
            "business earnings revenue profit stocks shares merger acquisition IPO dividends CEO layoffs tariffs GDP inflation hedge-fund private-equity startup venture-capital fiscal bonds FTSE Nasdaq shareholder buyout trade-deal commerce markets recession bookkeeping audit liquidation bankruptcy conglomerate arbitrage commodity-trading insolvency mercantile quarterly-results acquisition-bid hostile-takeover manufacturing supply-chain logistics factory automation textiles outsourcing macroeconomics balance-sheet amortization depreciation liquidity microfinance e-commerce wholesale ledger asset-valuation market-capitalization"
        }
        // Government, parliament, elections, political parties, and military/defence affairs
        Category::Politics => {
            "politics parliament MPs chancellor legislation elections Conservative Labour LibDems Reform cabinet minister Starmer Farage Reeves Sunak Trump Putin vote constituency manifesto polling Holyrood Whitehall PMQs SNP military army navy RAF defence war NATO troops Israel missile drone Treasury Downing-Street backbencher frontbench state-visit diplomacy geopolitics referendum electorate bicameral devolution incumbency Democrats Republicans Libertarians Mayor Congress Senate White-House Pentagon Kremlin Bundestag Elysée sanctions embassy treaty bipartisan gubernatorial impeachment autocracy federalism by-election turnout benefits welfare social-security DWP councillor legislature municipal executive-branch ballot-box geopolitical-alliance civil-service public-policy statehood"
        }
        // Courts, crime, police investigations, and criminal justice
        Category::Law => {
            "law court trial convicted jailed sentenced defendant prosecution verdict jury judge murder stabbing knife beating assault attack grooming exploitation abuse offender indecent coercive stalking fraud theft robbery ASA FCA banned misconduct DWP lawsuit acquitted injunction plaintiff caution probe tribunal litigation legal-action manslaughter burglary shoplifting extradition forensics bailiff subpoena affidavit perjury jurisprudence embezzlement investigation missing-person inquest coroner detective tragedy drowned rescuers tribute police-cordon rape custodial-sentence anonymity-order County-Court FTC judiciary legal-counsel arraignment statutory appeal-court penal-code cross-examination felony misdemeanor tort-law"
        }
        // Clinical medicine and personal health — NHS, diagnosis, treatment, fitness, diet
        Category::Health => {
            "health NHS hospital GP vaccine diagnosis cancer surgery prescription clinical trial symptoms disease epidemic inpatient psychiatric pharmaceutical medication therapy chronic disability overdose ward nurse stroke dementia autism mental health fitness diet nutrition exercise workout weight sleep supplements vitamins cardiology oncology immunology epidemiology neurology pathology radiology physiotherapy emergency-services first-responders paramedics palliative hospice melanoma triage outpatient preventive-medicine prognosis inpatient-care chronic-illness inoculation pathogen physical-therapy metabolic-health"
        }
        // Entertainment, film, TV, music, celebrities, streaming, and media
        Category::Culture => {
            "culture Netflix Disney streaming celebrities actor director Oscars BAFTA album concert band horror thriller documentary royals King Queen Royal entertainment Grammy Glastonbury blockbuster sitcom season episode premiere trailer how to watch film broadcaster Ofcom Channel 4 BBC media paparazzi showbiz cinematography choreography discography playwright red-carpet stardom influencer Hollywood Bollywood Cannes Sundance Eurovision K-pop Broadway West-End exhibition gallery museum sculpture literature bestseller casting audition cheese-rolling YouTubers festival headlining gnomes chart-topper rapper studio-album ITV Bafta novel author book discography theater box-office non-fiction biography main-stage performance-art showrunner"
        }
        // Domestic life, cooking, home, fashion, consumer tips, and personal finance
        Category::Lifestyle => {
            "lifestyle recipe cooking cleaning decor fashion wardrobe travel garden kitchen wedding household mattress bedding interior skincare bathroom laundry storage hacks tips ants vinegar coffee mortgage rent savings pension budget bills energy loan credit housing property landlord employment job unemployed graduate parenting housekeeping upholstery sourdough minimalism staycation decluttering novel author island-life wardens remote-living dating personal-finance meal-prep home-renovation fast-fashion thrifting parenting-tips skincare-routine baking textiles"
        }
        // Cars, transport infrastructure, aviation, and commuting
        Category::Transport => {
            "transport car EV electric vehicle Tesla road motorway highway aviation airline airport train railway commute driver fuel petrol diesel autonomous self-driving bus lorry van cycling infrastructure logistics freight aeronautics maritime-shipping locomotive fuselage tarmac air-traffic-control haulage tramway Port-of-Dover rolling-stock rapid-transit airspace shipping-lane interchange bypass multi-modal light-rail toll-road"
        }
        // Wildlife, ecology, the natural world, and climate — animals, plants, weather, conservation
        Category::Nature => {
            "nature wildlife animals birds insects plants trees forests oceans seas river mountains countryside national parks ecology biodiversity extinction endangered conservation habitat hedgehog deer butterfly orchid climate carbon flooding drought heatwave rewilding farming weather meteorology zoology botany entomology ornithology ecosystem reforestation precipitation humidity anticyclone pesticides waste-sites tonnes-of-waste spider insect fish butterfly shark elephant dog cat rabbit hamster flora fauna invasive-species migration-pattern biosphere wilderness wetlands nature-reserve marine-life deforestation"
        }
        // Consumer electronics and hardware — phones, laptops, TVs, headphones, wearables
        Category::Technology => {
            "technology smartphone iPhone Android laptop notebook tablet smartwatch wearable headphones on-ear over-ear earphones audio speakers display OLED television TV camera drone VPN router hardware gadget Apple Samsung LG Sony Google Meta Facebook Twitter Microsoft Huawei Lenovo IBM Oracle Nvidia AMD console GPU peripherals motherboard chipset bluetooth firmware-update pixel-density megapixel print-methods form-factor gigahertz lithium-ion form-factor liquid-cooling solid-state-drive chassis spec-sheet"
        }
        // Software development, coding, and cybersecurity
        Category::Software => {
            "software coding programming Python Ruby Rust JavaScript TypeScript HTML CSS web websites React Node GitHub API open-source Linux algorithm compiler binary firmware SDK cybersecurity breach hacking phishing malware ransomware exploit vulnerability CVE backend frontend devops refactoring middleware multithreading encryption-key repository containerization source-code library-dependency debugging IDE syntax database-schema version-control continuous-integration agile-development production-deploy"
        }
        // Artificial intelligence, machine learning, and AI assistants
        Category::AI => {
            "AI ChatGPT Claude Gemini GPT OpenAI Anthropic DeepMind machine-learning LLM neural-network model-training inference chatbot Copilot Midjourney Stable-Diffusion robotics generative-AI singularity transformer-architecture backpropagation reinforcement-learning weights-and-biases fine-tuning hallucinations prompt-engineering superintelligence web-scraping crawlers data-mining synthetic-data computer-vision NLP parameter-count embeddings tokenization overfitting autonomous-agents"
        }
        // Scientific research, astronomy, biology, and academic discovery
        Category::Science => {
            "science scientific study astronomy telescope genome species paleontology neuroscience botany physics quantum biology chemistry fossils stargazing celestial earthshine mathematics NASA rocket satellite spacecraft orbit Mars experiment journal hypothesis lab discovery space-exploration astrophysics particle-accelerator CRISPR relativity thermodynamics microbiology molecular-genetics exoplanet super-collider vaccine-trials lunar solar Artemis atmosphere space Higgs boson electron neutron chemicals nucleosynthesis lithosphere chromatography spectroscopy stoichiometry mitosis meiosis prokaryote eukaryote CRISPR-Cas9 phenotypic-trait genotype epigenetic phylogenetic photosynthesis angiosperm gymnosperm mycelium cosmic-microwave-background event-horizon magnetosphere ionosphere black-hole pulsar quasar neutron-star dark-matter dark-energy quantum-entanglement superposition wavefunction hadron boson fermion lepton quark neutrino plasma-physics entropy enthalpy seismology vulcanology paleoclimatology bioinformatics proteomics neurogenesis synaptic-plasticity neurotransmitter"
        }
        // Physical competitive sports and athletic leagues
        Category::Sports => {
            "sports football cricket tennis rugby golf boxing swimming cycling wicket Premier-League Champions-League Europa-League Bundesliga La-Liga NBA NFL MLB transfer-window squad batting Wimbledon Olympics F1 Grand-Prix athletics marathon heptathlon scoreboard goalscorer touchdown wicket-keeper grand-slam tie-break velodrome podium-finish paralympics World-Cup trophy national-team call-up offside leg-before-wicket deuce conversion backhand bogey peloton repechage decathlon steeplechase velodrome slip-stream paratriathlon biathlon drag-reduction-system pole-position yellow-card red-card sin-bin touchline scrum quarterback linebacker shortstop bullpen home-run innings crease googly yorker ace tiebreaker unforced-error volley birdie eagle par southpaw upper-cut pole-vault triple-jump pommel-horse draft-pick salary-cap play-offs wild-card golden-boot hat-trick clean-sheet relegation promotion VAR clean-and-jerk snatch lineout ruck maul puck blue-line power-play serve-and-volley topspin corner-man split-decision technical-knockout anti-doping WADA IOC FIFA ICC IAAF UEFA"
        }
        // Video games, gaming culture, esports, and game releases
        Category::Gaming => {
            "gaming game gameplay esports Fortnite Minecraft Steam PlayStation Xbox Nintendo Switch RPG FPS shooter strategy multiplayer co-op indie AAA Zelda Mario studio console handheld retrogaming speedrun microtransactions loot-box ray-tracing frame-rate walkthrough boss-fight mmorpg discord-server modding season-pass sandbox twitch-streamer local-coop cross-play haptic-feedback achievements loading-screen open-world"
        }
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

pub fn calculate_preference_score(embedding: &[f32], rated: &[(Rating, Vec<f32>)]) -> f32 {
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

    if weight > 0.0 { sum / weight } else { 0.5 }
}
