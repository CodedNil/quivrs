use crate::{
    server::database,
    shared::{Category, PendingSource, Region},
};
use anyhow::{Context, Result, anyhow, bail};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use itertools::Itertools;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fmt::Write,
    sync::{LazyLock, Mutex},
};
use strum::IntoEnumIterator;
use surrealdb::types::RecordId;
use tracing::info;

const MODEL: EmbeddingModel = EmbeddingModel::EmbeddingGemma300M;
pub const EMBEDDING_MODEL_NAME: &str = "EmbeddingGemma300M";

const fn category_label(category: Category) -> &'static [&'static str] {
    match category {
        // Corporate finance, stock markets, and macroeconomics
        Category::Business => &[
            "Corporate finance, business earnings reports, revenue, profits, stocks, shares, and market trading.",
            "Company mergers, acquisitions, takeovers, IPOs, corporate restructuring, and CEO layoffs.",
            "Macroeconomics, GDP data, inflation rates, interest rates, central bank tariffs, and national recessions.",
            "Venture capital, startup investments, private equity, hedge funds, and market capitalization.",
            "Employment market, job vacancies, retail industry leadership, and corporate warnings.",
            "Cryptocurrency, Bitcoin, Ethereum, blockchain, stablecoins, tokenomics, DeFi, and crypto exchanges.",
            "Banking, liquidity crisis, insolvency, commercial lenders, central banks, Wall Street, and fiscal policies.",
            "Supply chain logistics, manufacturing output, trade deficits, export tariffs, and global commerce hubs.",
        ],
        // Government, parliament, elections, political parties, and military/defence affairs
        Category::Politics => &[
            "Government politics, parliament debates, legislation, MPs, ministers, policies, and Whitehall decisions.",
            "General elections, political party campaigns, voting polls, manifestos, by-elections, and electorate dynamics.",
            "Military affairs, defense spending, army, navy, RAF, NATO, troop movements, and global warfare.",
            "Geopolitics, international diplomacy, state visits, foreign policy, and bilateral trade agreements.",
            "UK domestic political figures and commentary involving Starmer, Sunak, Farage, Reeves, or Downing Street.",
            "Public opinion polling, voting intentions, and demographic surveys on social or national issues.",
            "Local council by-election results, seat gains and losses, and political party vote shares.",
            "Political controversy, sexism in politics, and demands for apologies from candidates.",
            "Country intensifies strikes on neighbouring countries, many casualties as war escalates.",
            "Congress, Senate, White House, Washington, European Parliament, legislation veto, and partisan voting blocks.",
            "Geopolitical sanctions, trade blockades, diplomatic expulsions, and United Nations UN resolutions.",
            "Civil unrest, public protests, strikes, trade unions, walkouts, and mass demonstrations.",
        ],
        // Courts, crime, police investigations, and criminal justice
        Category::Law => &[
            "Court trials, criminal convictions, judges, juries, defense lawyers, prosecution, and legal verdicts.",
            "Police investigations, criminal justice, arrests, forensics, raids, and law enforcement probes.",
            "Violent crimes, murder, assault, stabbing, theft, robbery, fraud, embezzlement, and legal misconduct.",
            "Lawsuits, high court injunctions, litigation, tribunals, regulatory bans, and civil legal action.",
            "Prison sentences, custodial terms, and legal controversies.",
            "Extradition treaties, supreme court rulings, judicial reviews, constitutional law, and appeals.",
            "Cybercrime, ransomware cartels, financial money laundering, cartel operations, and wire fraud indictment.",
        ],
        // Clinical medicine and personal health — NHS, diagnosis, treatment, fitness, diet
        Category::Health => &[
            "Clinical medicine, hospital operations, surgeries, emergency departments, wards, and NHS healthcare systems.",
            "Medical diagnoses, cancer treatments, chronic diseases, prescription medications, pharmaceuticals, and symptoms.",
            "Public health, virus epidemics, pathogen outbreaks, vaccinations, and epidemiology tracking.",
            "Personal fitness, nutrition, diet plans, workout routines, mental health therapy, and wellness.",
            "Medical warnings, health risks of social media, addiction, and doctor recommendations.",
            "Neuroscience, Alzheimer, dementia, mental illnesses, clinical depression, and psychological psychiatry.",
            "Pharmaceutical patents, clinical drug trials, FDA approval, medical research labs, and vaccines.",
            "Longevity science, biohacking, vitamins, metabolic health, diabetes treatments, and weight-loss drugs.",
        ],
        // Entertainment, film, TV, music, celebrities, streaming, and media
        Category::Culture => &[
            "Entertainment media, streaming shows, TV series episodes, seasonal premieres, trailers, and broadcasters.",
            "Celebrity culture, pop stars, actors, red carpet events, paparazzi, influencers, and Hollywood showbiz.",
            "Music albums, concerts, bands, music festivals, Glastonbury, and industry awards like the Grammys or BAFTAs.",
            "Fine arts, theatre productions, literature, novel authors, museum exhibitions, and cultural reviews.",
            "TV personalities, presenters, reality show contestants, and broadcasting careers.",
            "Netflix, Disney+, HBO, Paramount, streaming viewership ratings, box office ticket sales, and cinematic universes.",
            "YouTube creators, TikTok influencers, viral memes, podcast series, and digital internet culture.",
        ],
        // Domestic life, cooking, home, fashion, consumer tips, and personal finance
        Category::Lifestyle => &[
            "Home cooking, baking recipes, meal prep, culinary tips, and kitchen design.",
            "Interior decor, cleaning hacks, household management, gardening, and property maintenance.",
            "Personal finance, mortgages, rent, household energy bills, savings, pensions, and budgeting advice.",
            "Fashion trends, wardrobe styles, skincare routines, dating life, parenting, and travel staycations.",
            "Consumer shopping deals, holiday sales, product discounts, mattress offers, and retail bargains.",
            "Workplace culture, office life, team-building, and career advice.",
            "Local festivals, farm shops, community events, and rural attractions.",
            "Property market, house hunting, real estate listings, Airbnb hosting, and boutique hotel reviews.",
            "Luxury craftsmanship, streetwear brands, cosmetics, beauty skincare, and runway fashion weeks.",
        ],
        // Cars, transport infrastructure, aviation, and commuting
        Category::Transport => &[
            "Electric vehicles, EVs, consumer cars, automotive engineering, roads, and motorway infrastructure.",
            "Aviation industry, airlines, airport operations, commercial flights, and airspace management.",
            "Rail networks, commuter trains, subways, public transit systems, and cargo freight shipping.",
            "Maritime container shipping, port bottlenecks, rail freight corridors, and delivery distribution networks.",
            "Autonomous self-driving, Tesla autopilot, hyperloop, commercial drones, and micro-mobility scooters.",
        ],
        // Wildlife, ecology, the natural world, and climate — animals, plants, weather, conservation
        Category::Nature => &[
            "Wildlife conservation, natural ecosystems, biodiversity, endangered animal species, and birds.",
            "Climate change, global warming, carbon emissions, extreme weather events, floods, and droughts.",
            "Environmental science, forestry, meteorology, countryside ecology, and marine biology.",
            "Weather forecasts, temperature records, heatwaves, rainfall, and meteorological conditions.",
            "Impact of extreme heat on housing, air conditioning trends, and climate adaptation.",
            "Pollution in rivers, waste water, sewage, and environmental damage to natural habitats.",
            "Renewable green energy, solar grids, wind farms, geothermal projects, and recycling infrastructure.",
            "Earthquakes, volcanic eruptions, seismic tremors, tsunamis, and geological fault lines.",
            "Ancient rainforest restoration, woodland expansion, and rewilding projects.",
        ],
        // Consumer electronics and hardware — phones, laptops, TVs, headphones, wearables
        Category::Technology => &[
            "Consumer electronics, smartphones, laptops, hardware components, and gadget specifications.",
            "Computer hardware, microchips, GPUs, motherboards, solid-state drives, and display panels.",
            "Smart home automation, wearable devices, audio speakers, and wireless routing peripherals.",
            "OLED TVs, display technology, screen burn-in, and consumer audio equipment.",
            "Semiconductor manufacturing, TSMC, Intel, Nvidia hardware architecture, and silicon fabs.",
            "Virtual reality, VR headsets, augmented reality, AR smart glasses, and spatial computing hardware.",
        ],
        // Software development, coding, and cybersecurity
        Category::Software => &[
            "Software development, coding practices, programming languages like Python, Rust, or JavaScript.",
            "Cybersecurity breaches, hacking incidents, malware exploits, ransomware, and system vulnerabilities.",
            "DevOps infrastructure, database architecture, open-source repositories, APIs, and cloud services.",
            "Internet privacy, VPNs, data protection, digital surveillance, and online security mitigations.",
            "Operating systems, Linux distributions, and software licensing or regulation.",
            "AWS, Microsoft Azure, Google Cloud, Docker containerization, Kubernetes clustering, and microservices.",
            "Git version control, GitHub code repositories, CI/CD automated deployment pipelines, and technical documentation.",
            "Digital supplier takeovers, online identification platforms, and critical digital infrastructure.",
        ],
        // Artificial intelligence, machine learning, and AI assistants
        Category::AI => &[
            "Artificial intelligence, machine learning, large language models, LLMs, and neural networks.",
            "Generative AI tools, chatbots like ChatGPT or Claude, prompt engineering, and image synthesis.",
            "AI safety research, model training, parameter counts, and autonomous agent development.",
            "AI assistants, and software features powered by artificial intelligence.",
            "OpenAI, Anthropic, DeepMind, Midjourney, transformer architecture, fine-tuning, and weights training datasets.",
            "Computer vision, natural language processing NLP, reinforcement learning, and token generation benchmarks.",
        ],
        // Scientific research, astronomy, biology, and academic discovery
        Category::Science => &[
            "Scientific research papers, laboratory experiments, breakthroughs, hypotheses, and academic journals.",
            "Space exploration, astronomy, telescopes, NASA rocket launches, satellites, and planetary discoveries.",
            "Theoretical physics, quantum mechanics, chemistry equations, biology, genetics, and molecular research.",
            "Archaeology, historical research, maritime history, and identifying ancient remains.",
            "CRISPR gene editing, DNA sequencing, molecular biophysics, and evolutionary paleontology.",
            "CERN particle accelerators, dark matter physics, black hole telemetry, and astrophysics calculations.",
        ],
        // Physical competitive sports and athletic leagues
        Category::Sports => &[
            "Football match reports, Premier League fixtures, Champions League results, World Cup, goals, and team lineups.",
            "Competitive athletic sports, cricket wickets, tennis grand slams, rugby scrums, and golf tournaments.",
            "Sports punditry, manager selections, team of the season reviews, transfer window signings, and squads.",
            "The Olympic games, professional athletes, marathons, motorsports F1 Grand Prix, and podium finishes.",
            "Grand Prix driver standings, race results, sporting events.",
            "NBA basketball playoffs, NFL Super Bowl touchdowns, baseball MLB, and athlete contract drafting.",
            "UFC MMA combat fighting, heavyweight boxing title belts, and professional athletics doping trials.",
            "Spectators injured in sporting accident, motorbike collides with spectators watching race.",
            "French Open, Paris heatwave tennis, athlete performance in extreme heat.",
        ],
        // Video games, gaming culture, esports, and game releases
        Category::Gaming => &[
            "Video game releases, gameplay mechanics, console hardware, and esports tournaments.",
            "Gaming culture, streaming channels, online multiplayer matches, and indie studio developments.",
            "PlayStation, Xbox Series, Nintendo Switch, Steam deck, PC gaming rigs, and retro retro-gaming emulators.",
            "MMORPG servers, competitive shooter matchmaking, speedrunning records, and gaming modding communities.",
        ],
    }
}

const fn region_label(region: Region) -> &'static str {
    match region {
        Region::Worldwide => {
            "global, worldwide, international, multilateral, transnational, global economy, global climate, United Nations, UN, G7, G20, multiple countries, across continents, global summit"
        }
        Region::UnitedKingdom => {
            "United Kingdom, UK, Britain, British, Great Britain, Westminster, Whitehall, Downing Street, House of Commons, House of Lords, HM Treasury, Bank of England, UK-wide"
        }
        Region::England => {
            "England, English, London, Greater London, Manchester, Birmingham, Liverpool, Leeds, Bristol, Newcastle, Yorkshire, Cornwall, Essex, Kent, English councils, English schools"
        }
        Region::Scotland => {
            "Scotland, Scottish, Scots, Edinburgh, Glasgow, Aberdeen, Dundee, Highlands, Holyrood, Scottish Parliament, Scottish Government, Scotland-wide"
        }
        Region::Wales => {
            "Wales, Welsh, Cymru, Cardiff, Swansea, Newport, Wrexham, Senedd, Welsh Government, Welsh language"
        }
        Region::NorthernIreland => {
            "Northern Ireland, Northern Irish, Belfast, Derry, Londonderry, Stormont, Ulster, Northern Ireland Assembly, Good Friday Agreement, Irish Sea border"
        }
        Region::Ireland => {
            "Ireland, Irish, Republic of Ireland, Dublin, Cork, Galway, Limerick, Dail Eireann, Taoiseach, Irish government"
        }
        Region::UnitedStates => {
            "United States, USA, US, American, Washington DC, White House, Congress, Senate, Supreme Court, federal, California, Texas, New York, Florida, Pentagon, Wall Street"
        }
        Region::Canada => {
            "Canada, Canadian, Ottawa, Toronto, Vancouver, Montreal, Quebec, Alberta, Ontario, British Columbia, Canadian parliament, RCMP, Bank of Canada"
        }
        Region::NorthAmerica => {
            "North America, North American, continental, NAFTA, USMCA, cross-border trade, transcontinental, continental border, Great Lakes, North American Arctic, regional supply chain"
        }
        Region::LatinAmerica => {
            "Latin America, Latin American, South America, Central America, Caribbean, Brazil, Brasilia, Argentina, Buenos Aires, Chile, Santiago, Colombia, Bogota, Peru, Lima, Venezuela, Mexico, Cuba, Haiti, Mercosur"
        }
        Region::WesternEurope => {
            "Western Europe, European Union, EU, eurozone, Brussels, France, French, Paris, Germany, German, Berlin, Spain, Madrid, Italy, Rome, Netherlands, Amsterdam, Belgium, Austria, Switzerland, Portugal, Nordic, Denmark, Sweden, Norway"
        }
        Region::EasternEurope => {
            "Eastern Europe, eastern European, Ukraine, Kyiv, Poland, Warsaw, Romania, Bucharest, Hungary, Budapest, Czechia, Prague, Slovakia, Balkans, Serbia, Croatia, Bulgaria, Moldova, Baltic, Estonia, Latvia, Lithuania, Belarus"
        }
        Region::MiddleEastNorthAfrica => {
            "Middle East, North Africa, MENA, Israel, Israeli, Jerusalem, Gaza, Palestine, Palestinian, West Bank, Iran, Tehran, Iraq, Baghdad, Syria, Damascus, Lebanon, Beirut, Jordan, Amman, Saudi Arabia, Riyadh, UAE, Dubai, Qatar, Gulf, Egypt, Cairo, Morocco, Algeria, Tunisia, Libya"
        }
        Region::SubSaharanAfrica => {
            "sub-Saharan Africa, African Union, Nigeria, Abuja, Lagos, Kenya, Nairobi, South Africa, Pretoria, Johannesburg, Ethiopia, Addis Ababa, Ghana, Accra, Congo, DRC, Kinshasa, Sudan, Uganda, Tanzania, Zimbabwe, Senegal, Rwanda"
        }
        Region::SouthAsia => {
            "South Asia, India, Indian, New Delhi, Mumbai, Pakistan, Pakistani, Islamabad, Bangladesh, Dhaka, Sri Lanka, Colombo, Nepal, Kathmandu, Afghanistan, Kabul, Bhutan, Maldives, Kashmir, Punjab"
        }
        Region::EastAsia => {
            "East Asia, China, Chinese, Beijing, Shanghai, Japan, Japanese, Tokyo, South Korea, Korean, Seoul, North Korea, Pyongyang, Taiwan, Taiwanese, Taipei, Hong Kong, Macau, Mongolia, East China Sea"
        }
        Region::SoutheastAsia => {
            "Southeast Asia, ASEAN, Indonesia, Jakarta, Vietnam, Hanoi, Thailand, Bangkok, Philippines, Manila, Malaysia, Kuala Lumpur, Singapore, Myanmar, Burma, Cambodia, Laos, Brunei, Timor-Leste, South China Sea"
        }
        Region::CentralAsia => {
            "Central Asia, Kazakhstan, Astana, Almaty, Uzbekistan, Tashkent, Kyrgyzstan, Bishkek, Tajikistan, Dushanbe, Turkmenistan, Ashgabat, Caspian, Silk Road, central Asian"
        }
        Region::Oceania => {
            "Oceania, Australia, Australian, Canberra, Sydney, Melbourne, New Zealand, Wellington, Auckland, Pacific Islands, Fiji, Papua New Guinea, Samoa, Tonga, Solomon Islands, Tasmania"
        }
    }
}

pub fn sentiment_label(sentiment: &str) -> &'static str {
    match sentiment {
        // Uplifting, solutions-oriented, or curiosity-inducing framing
        "positive" => {
            "positive milestone breakthrough triumph progress success solution forward heartwarming unity inspiring resilience recovery champion recovery masterpiece miracle innovation discovery genius curiosity fascinating mystery secret unexpected intriguing awe inspiring milestone unique pioneer brilliant boost stellar incredible superb outstanding together community harmony optimism win winner gain recovery growth peace stability safety exoneration"
        }
        // Cynical, critical, aggressive, or tragic framing
        "negative" => {
            "negative toxic outrage controversy backlash condemnation failure disaster scandal fury brutal devastating threat bleak warning critical crisis tragedy investigation slammed gridlock error lawsuit dispute worst hostile bleak chaos bitter panic blame ruined fault shocking horrific collapse dangerous failure bleeding nightmare warning violence felony assault victim trauma grief death killed stabbed shot abuse crime jail prison convict embezzlement stolen theft fraud murder killing sexist lewd apology betrayal Nazi hateful accident abortion sexual offences abuse conviction manslaughter embezzlement corruption scandal"
        }
        _ => "",
    }
}

pub fn importance_label(importance: &str) -> &'static str {
    match importance {
        // Macro-scale structural changes, high-consequence policy, and permanence
        "important" => {
            "important, historic, monumental, unprecedented, permanent, landmark, fundamental, crisis, systemic, global, national, macro, widespread, structural, existential, priority, turning-point, paradigm-shift, definitive, far-reaching, critical, essential, massive, long-term, key-factor, major-overhaul, catalyst, strategic, sweeping, emergency, tectonic-shift, paramount, election-result, conviction, sentencing, war-strike, geopolitical-shift, embezzlement-scandal, murder-trial, historic-offences, national-security, constitutional-crisis, general-election-result"
        }
        // Micro-scale, transient, or routine consumer/entertainment news
        "unimportant" => {
            "unimportant, routine, minor, transient, local, niche, consumer-deal, retail-sale, shopping-discount, product-review, gadget-unboxing, daily-update, weather-forecast, minor-fixture, gossip, celebrity-sighting, casual-mention, hobbyist-tip, routine-maintenance, temporary-offer, limited-time-deal, bargain, coupon, discount, price-drop, clearance, flash-sale, interview, documentary, profile, personal-story, human-interest, feature-article, streaming-guide, tv-recommendation, workout-tip, exercise-routine, pillow, mattress-sale, hair-styler-review, smart-lights, burger"
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

struct LabelDefinition {
    key: String,
    label_group: &'static str,
    label_value: String,
    hash: String,
    text: &'static str,
}

fn label_definitions() -> Vec<LabelDefinition> {
    let mut definitions = Vec::new();

    for category in Category::iter() {
        for (idx, text) in category_label(category).iter().enumerate() {
            definitions.push(LabelDefinition {
                key: format!("category:{category}:{idx}"),
                label_group: "category",
                label_value: category.to_string(),
                hash: label_hash(text),
                text,
            });
        }
    }

    for region in Region::iter() {
        let text = region_label(region);
        definitions.push(LabelDefinition {
            key: format!("region:{region}"),
            label_group: "region",
            label_value: region.to_string(),
            hash: label_hash(text),
            text,
        });
    }

    for (group, key, text) in [
        ("sentiment", "positive", sentiment_label("positive")),
        ("sentiment", "negative", sentiment_label("negative")),
        ("importance", "important", importance_label("important")),
        ("importance", "unimportant", importance_label("unimportant")),
    ] {
        definitions.push(LabelDefinition {
            key: format!("{group}:{key}"),
            label_group: group,
            label_value: key.to_string(),
            hash: label_hash(text),
            text,
        });
    }

    definitions
}

static EMBEDDING_MODEL: LazyLock<Mutex<TextEmbedding>> = LazyLock::new(|| {
    Mutex::new(
        TextEmbedding::try_new(InitOptions::new(MODEL).with_show_download_progress(true))
            .expect("Failed to load embedding model"),
    )
});

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
    if texts.is_empty() {
        return Ok(Vec::new());
    }

    let texts = texts.to_vec();
    let mut embeddings = tokio::task::spawn_blocking(move || {
        EMBEDDING_MODEL
            .lock()
            .map_err(|_| anyhow!("Embedding model lock poisoned"))?
            .embed(texts, None)
    })
    .await
    .context("Embedding worker task failed")??;

    for embedding in &mut embeddings {
        normalize(embedding);
    }
    Ok(embeddings)
}

fn normalize(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v {
            *x /= norm;
        }
    }
}

async fn maintenance_label_embeddings() -> Result<()> {
    let definitions = label_definitions();
    let keys: HashSet<_> = definitions.iter().map(|def| def.key.clone()).collect();
    let cached_hashes = database::get_label_hashes().await?;
    let stale: Vec<_> = definitions
        .iter()
        .filter(|def| cached_hashes.get(&def.key) != Some(&def.hash))
        .collect();

    if !stale.is_empty() {
        info!("Refreshing {} label embeddings", stale.len());
        let texts: Vec<_> = stale.iter().map(|def| def.text.to_string()).collect();
        let embeddings = generate_article_embeddings(&texts).await?;
        let records: Vec<_> = stale
            .into_iter()
            .zip(embeddings)
            .map(|(def, embedding)| database::LabelEmbeddingRecord {
                id: RecordId::new("label_embeddings", def.key.as_str()),
                label_group: def.label_group.to_string(),
                label_value: def.label_value.clone(),
                hash: def.hash.clone(),
                text: def.text.to_string(),
                embedding,
            })
            .collect();

        database::upsert_label_embeddings(&records).await?;
    }

    database::delete_label_embeddings_except(&keys).await?;
    Ok(())
}

pub async fn maintenance_embeddings() -> Result<()> {
    maintenance_label_embeddings().await?;
    maintenance_article_embeddings().await
}

async fn maintenance_article_embeddings() -> Result<()> {
    let current_model = EMBEDDING_MODEL_NAME;

    for table in ["pending_sources", "user_articles"] {
        let stale = database::get_stale_embedding_records(table, current_model).await?;
        if stale.is_empty() {
            continue;
        }

        info!("Updating embeddings for {} records in {table}", stale.len());
        for chunk in stale.chunks(100) {
            let texts: Vec<String> = chunk.iter().map(|r| r.embedding_text.clone()).collect();
            let new_embeddings = generate_article_embeddings(&texts).await?;
            let updates: Vec<_> = chunk
                .iter()
                .zip(new_embeddings)
                .map(|(record, embedding)| database::EmbeddingUpdate {
                    id: record.id.clone(),
                    embedding,
                })
                .collect();
            database::update_record_embeddings(&updates, current_model).await?;
        }
    }

    Ok(())
}

pub async fn classify(article_embedding: &[f32]) -> Result<(Category, Region, f32, f32)> {
    let scores = database::get_label_scores(article_embedding).await?;

    Ok((
        best_label(&scores, "category")?,
        best_label(&scores, "region")?,
        binary_label_score(&scores, "sentiment", "positive")?,
        binary_label_score(&scores, "importance", "important")?,
    ))
}

fn best_label<T>(scores: &[database::LabelScore], group: &str) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    scores
        .iter()
        .filter(|row| row.label_group == group)
        .max_by(|a, b| a.similarity.total_cmp(&b.similarity))
        .ok_or_else(|| anyhow!("No {group} label embeddings found"))?
        .label_value
        .parse()
        .map_err(Into::into)
}

fn binary_label_score(
    rows: &[database::LabelScore],
    label_group: &str,
    positive_value: &str,
) -> Result<f32> {
    let mut positive = 0.0;
    let mut total = 0.0;
    for row in rows.iter().filter(|row| row.label_group == label_group) {
        let weight = (row.similarity * 10.0).exp();
        if row.label_value == positive_value {
            positive += weight;
        }
        total += weight;
    }

    if total == 0.0 {
        bail!("No {label_group} label embeddings found");
    }

    Ok(positive / total)
}
