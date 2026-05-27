use crate::shared::Category;

pub const fn labels(category: Category) -> &'static [&'static str] {
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
            "An IPO, share sale, or flotation involving a commercial space or technology company.",
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
            "National polling, voting intention, party leadership challenges, council by-elections, and election candidates.",
            "Government decisions about school policy, public regulation, or national public services.",
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
            "A local emergency report about a fatal fall, stabbing, fire, explosion, missing person, or body recovered from water.",
            "An investigation into leaked exams, unlawful conduct, regulatory breaches, or evidence before a court.",
            "A court ruling on consumer rights, a criminal prosecution, or a police arrest after a serious offence.",
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
            "A story about a musician, jazz legend, obituary, synthesizer, or electronic musical instrument.",
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
            "Household electricity price caps, domestic energy bills, and consumer cost-of-living advice.",
        ],
        // Cars, transport infrastructure, aviation, and commuting
        Category::Transport => &[
            "Electric vehicles, EVs, consumer cars, automotive engineering, roads, and motorway infrastructure.",
            "Aviation industry, airlines, airport operations, commercial flights, and airspace management.",
            "Rail networks, commuter trains, subways, public transit systems, and cargo freight shipping.",
            "Maritime container shipping, port bottlenecks, rail freight corridors, and delivery distribution networks.",
            "Autonomous self-driving, Tesla autopilot, hyperloop, commercial drones, and micro-mobility scooters.",
            "A road closure, burning lorry, railway connectivity upgrade, airline Wi-Fi rollout, or transport disruption.",
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
            "Fitness trackers, smart watches, sleep tracking rings, action cameras, and consumer wearable comparisons.",
            "A physical technology product such as a phone camera accessory, touchscreen laptop add-on, or music synthesizer.",
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
            "Privacy and security implications of metadata collection, encryption access, VPN products, or data retention law.",
            "Desktop applications, clipboard managers, GNOME extensions, Rust utilities, and open-source developer tools.",
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
            "Spaceflight research, astronauts, Moon base missions, and exercise equipment designed for use in space.",
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
