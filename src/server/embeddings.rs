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
const CLASSIFICATION_PROMPT: &str = "task: classification | query: ";

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
            "News about migration policy, asylum crossings, border control, deportation, or refugees.",
            "News about government regulation of online platforms or protections for children online.",
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
            "A police report or inquest about a shooting, drowning, fatal incident, or suspicious death.",
            "A criminal justice story about rape, child sexual exploitation, domestic abuse, or safeguarding failures.",
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
            "A music story about a band, concert, live performance, album, or touring artist.",
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
            "A weather story about record temperatures, heatwaves, flooding, rainfall, or climate extremes.",
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
            "Motorsport competitions, Formula One racing, Isle of Man TT events, circuits, riders, and race results.",
            "French Open tennis matches, tournament draws, players, athletes, and sporting performance.",
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

const fn region_labels(region: Region) -> &'static [&'static str] {
    match region {
        Region::Worldwide => &[
            "An international story involving several countries or a global institution.",
            "A story with no specified geographic location or regional focus.",
            "A consumer technology or entertainment story released for a global audience.",
            "A worldwide business, science, or internet story with global relevance.",
        ],
        Region::UnitedKingdom => &[
            "A national story about Britain or the United Kingdom as a whole.",
            "A UK government story involving Westminster, Downing Street, or Parliament.",
            "A story about British public opinion, the UK economy, or a UK-wide institution.",
            "A policy or event affecting people across the nations of the United Kingdom.",
        ],
        Region::England => &[
            "A story about England or the English national government and public services.",
            "A local story in London, Manchester, Birmingham, Liverpool, or another English city.",
            "A story about an English county, council, school, court, river, or community.",
            "A story about elections, policing, weather, or infrastructure specifically in England.",
        ],
        Region::Scotland => &[
            "A national story about Scotland or the Scottish public.",
            "A political story involving Holyrood or the Scottish Government.",
            "A local story in Edinburgh, Glasgow, Aberdeen, Dundee, or the Highlands.",
            "A Scottish story about courts, councils, nature, business, or public services.",
        ],
        Region::Wales => &[
            "A national story about Wales or the Welsh public.",
            "A political story involving the Senedd or Welsh Government.",
            "A local story in Cardiff, Swansea, Newport, Wrexham, or a Welsh community.",
            "A Welsh story about policing, weather, transport, education, or public services.",
        ],
        Region::NorthernIreland => &[
            "A national story about Northern Ireland or the Northern Irish public.",
            "A political story involving Stormont or the Northern Ireland Assembly.",
            "A local story in Belfast, Derry, County Antrim, or another Northern Irish place.",
            "A Northern Ireland story about courts, policing, nature, health, or public services.",
        ],
        Region::Ireland => &[
            "A national story about the Republic of Ireland or the Irish public.",
            "A government story involving Dublin, the Dail, or the Taoiseach.",
            "A local story in Dublin, Cork, Galway, Limerick, or another Irish place.",
            "An Irish story about business, courts, health, culture, or public services.",
        ],
        Region::UnitedStates => &[
            "A national story about the United States or the American public.",
            "A US government story involving Washington, Congress, the White House, or federal agencies.",
            "A local story in an American state or city such as California, Texas, or New York.",
            "A US story about business, courts, transport, schools, or public policy.",
        ],
        Region::Canada => &[
            "A national story about Canada or the Canadian public.",
            "A Canadian government story involving Ottawa or Parliament.",
            "A local story in Ontario, Quebec, Alberta, British Columbia, Toronto, or Montreal.",
            "A Canadian story about business, courts, climate, health, or public services.",
        ],
        Region::NorthAmerica => &[
            "A North American story affecting both the United States and Canada.",
            "A continental trade or transport story spanning North American borders.",
            "A regional climate or energy story across North America.",
            "A story explicitly about North American markets, sport, or industry.",
        ],
        Region::LatinAmerica => &[
            "A regional story about Latin America, Central America, South America, or the Caribbean.",
            "A national story about Brazil, Argentina, Chile, Colombia, Peru, or Venezuela.",
            "A story about Mexico, Cuba, Haiti, or another Caribbean or Central American country.",
            "A Latin American story about elections, economy, security, climate, or culture.",
        ],
        Region::WesternEurope => &[
            "A Western European or European Union story involving Brussels or EU institutions.",
            "A national story about France, Germany, Spain, Italy, or Portugal.",
            "A story about the Netherlands, Belgium, Austria, Switzerland, or Nordic Europe.",
            "A western European story about policy, economy, security, climate, or culture.",
        ],
        Region::EasternEurope => &[
            "An eastern European story involving Ukraine, Kyiv, Russia, or Moscow.",
            "A national story about Poland, Romania, Hungary, Czechia, or Slovakia.",
            "A story involving Moldova, Belarus, the Baltics, or the Balkans.",
            "An eastern European story about war, security, politics, economy, or society.",
        ],
        Region::MiddleEastNorthAfrica => &[
            "A Middle Eastern story involving Israel, Palestine, Iran, Iraq, Syria, or Lebanon.",
            "A Gulf story involving Saudi Arabia, the United Arab Emirates, Qatar, or Yemen.",
            "A North African story involving Egypt, Morocco, Algeria, Tunisia, or Libya.",
            "A MENA story about conflict, diplomacy, energy, society, business, or culture.",
        ],
        Region::SubSaharanAfrica => &[
            "A sub-Saharan African story involving Nigeria, Ghana, Senegal, or west Africa.",
            "An east African story involving Kenya, Ethiopia, Uganda, Tanzania, or Rwanda.",
            "A southern or central African story involving South Africa, Zimbabwe, or Congo.",
            "An African story south of the Sahara about politics, health, business, climate, or society.",
        ],
        Region::SouthAsia => &[
            "A South Asian story involving India, New Delhi, Mumbai, or Indian institutions.",
            "A story involving Pakistan, Bangladesh, Sri Lanka, Nepal, Bhutan, or the Maldives.",
            "A regional story about South Asian politics, economy, security, climate, or society.",
            "A story located in Afghanistan, Kashmir, Punjab, or elsewhere in the South Asian region.",
        ],
        Region::EastAsia => &[
            "An East Asian story involving China, Beijing, Shanghai, or Hong Kong.",
            "A Japanese story involving Tokyo or Japanese public institutions and companies.",
            "A Korean story involving South Korea, North Korea, Seoul, or Pyongyang.",
            "A regional story involving Taiwan, Mongolia, East Asian politics, business, or culture.",
        ],
        Region::SoutheastAsia => &[
            "A Southeast Asian story involving Indonesia, Vietnam, Thailand, or the Philippines.",
            "A story involving Singapore, Malaysia, Myanmar, Cambodia, Laos, or Brunei.",
            "A regional story involving ASEAN or the South China Sea.",
            "A southeast Asian story about government, economy, security, climate, or culture.",
        ],
        Region::CentralAsia => &[
            "A Central Asian story involving Kazakhstan or Uzbekistan.",
            "A story involving Kyrgyzstan, Tajikistan, or Turkmenistan.",
            "A regional story involving the Caspian Sea or central Asian trade routes.",
            "A central Asian story about politics, security, energy, economy, or society.",
        ],
        Region::Oceania => &[
            "An Australian story involving Canberra, Sydney, Melbourne, or Australian institutions.",
            "A New Zealand story involving Wellington, Auckland, or New Zealand institutions.",
            "A Pacific islands story involving Fiji, Samoa, Tonga, or Papua New Guinea.",
            "An Oceania story about politics, climate, business, health, sport, or culture.",
        ],
    }
}

fn sentiment_labels(sentiment: &str) -> &'static [&'static str] {
    match sentiment {
        "positive" => &[
            "A story celebrating a successful achievement or award.",
            "A medical story reporting recovery or a treatment improving patients' lives.",
            "A rescue or safety effort that protected people from harm.",
            "A peace agreement or constructive cooperation resolving a dispute.",
            "A conservation project restoring habitats or protecting wildlife.",
            "A useful invention or scientific breakthrough providing clear benefits.",
            "A sporting victory or outstanding performance presented with admiration.",
            "An economic improvement creating jobs or helping households.",
            "A legal outcome protecting victims or correcting an injustice.",
            "A community donation or volunteer effort supporting people in need.",
            "A strongly favourable review describing satisfaction with a product or experience.",
            "A hopeful development offering progress after a difficult situation.",
        ],
        "negative" => &[
            "A tragic story about a death, fatal accident, or bereavement.",
            "A violent crime story involving assault, rape, abuse, or exploitation.",
            "A corruption or fraud scandal involving wrongdoing and conviction.",
            "A war story reporting attacks, escalation, destruction, or casualties.",
            "An environmental story reporting pollution or serious damage to wildlife.",
            "A public service failure leaving vulnerable people at risk.",
            "A business story about job losses, bankruptcy, or financial collapse.",
            "A political story marked by bitter conflict, condemnation, or backlash.",
            "A health story describing serious disease, unsafe conditions, or preventable harm.",
            "An accident or disaster causing injuries, evacuation, or severe disruption.",
            "A story reporting discrimination, human rights violations, or intimidation.",
            "A negative review describing a defective product or disappointing experience.",
        ],
        _ => &[],
    }
}

fn importance_labels(importance: &str) -> &'static [&'static str] {
    match importance {
        "important" => &[
            "A national law or major government policy that changes people's rights or services.",
            "A general election, referendum, constitutional decision, or transfer of political power.",
            "An armed conflict, military strike, ceasefire, sanction, or major diplomatic crisis.",
            "A significant criminal trial, conviction, sentence, public inquiry, or justice-system failure.",
            "A fatal incident or public safety emergency affecting several people or exposing serious risk.",
            "A national health threat, medical breakthrough, or large-scale safeguarding issue.",
            "A major environmental disaster, climate record, pollution event, or conservation decision.",
            "A major economic change, financial crisis, corporate collapse, or employment shock.",
            "A critical transport, energy, communications, or public infrastructure development.",
            "A major scientific or technological milestone with broad societal consequences.",
            "A systemic abuse, civil rights, child safety, or human rights development.",
            "A major regulator, court, or government action affecting an industry or large population.",
        ],
        "unimportant" => &[
            "A shopping discount, seasonal sale, coupon, price drop, or bargain recommendation.",
            "A consumer review of a mattress, pillow, appliance, wearable, or household product.",
            "A guide recommending films, television shows, or streaming releases to watch.",
            "A lifestyle feature offering everyday home, fashion, dating, or wellness advice.",
            "A workout routine, fitness tip, or personal exercise recommendation.",
            "A gardening, cooking, decorating, cleaning, or home improvement tip.",
            "A routine sports preview, transfer rumour, fan survey, or minor fixture update.",
            "A light celebrity, entertainment, music, or popular culture feature.",
            "A small gadget accessory announcement or incremental consumer product launch.",
            "A promotional travel offer, shopping list, gift guide, or buying guide.",
            "A personal profile, opinion piece, interview, or human-interest feature without wider impact.",
            "A niche hobby update, casual tutorial, game recommendation, or routine leisure story.",
        ],
        _ => &[],
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
                hash: label_hash(&classification_input(text)),
                text,
            });
        }
    }

    for region in Region::iter() {
        for (idx, text) in region_labels(region).iter().enumerate() {
            definitions.push(LabelDefinition {
                key: format!("region:{region}:{idx}"),
                label_group: "region",
                label_value: region.to_string(),
                hash: label_hash(&classification_input(text)),
                text,
            });
        }
    }

    for (group, key, texts) in [
        ("sentiment", "positive", sentiment_labels("positive")),
        ("sentiment", "negative", sentiment_labels("negative")),
        ("importance", "important", importance_labels("important")),
        (
            "importance",
            "unimportant",
            importance_labels("unimportant"),
        ),
    ] {
        for (idx, text) in texts.iter().enumerate() {
            definitions.push(LabelDefinition {
                key: format!("{group}:{key}:{idx}"),
                label_group: group,
                label_value: key.to_string(),
                hash: label_hash(&classification_input(text)),
                text,
            });
        }
    }

    definitions
}

pub fn classification_input(text: &str) -> String {
    format!("{CLASSIFICATION_PROMPT}{text}")
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
        let texts: Vec<_> = stale
            .iter()
            .map(|def| classification_input(def.text))
            .collect();
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
    maintenance_article_embeddings().await?;
    Ok(())
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
