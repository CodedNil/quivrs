use crate::shared::Category;

pub const fn labels(category: Category) -> &'static [&'static str] {
    match category {
        // Companies, money, employment, markets, and the economy.
        Category::Business => &[
            "Business and economics reporting about companies, corporate strategy, profits, earnings, investment, ownership, mergers, acquisitions, bankruptcy, or commercial performance.",
            "Financial markets and financial services: stocks, shares, bonds, banking, lending, mortgages, pensions, insurance, fintech, cryptocurrency, IPOs, and investor decisions.",
            "The economy and household finances: inflation, interest rates, taxation, wages, energy prices, living costs, economic growth, trade, productivity, and consumer spending.",
            "Employment and enterprise: jobs, salaries, redundancies, industrial relations, entrepreneurship, workplace policy, skills, careers, and business investment.",
            "Property finance and major consumer financial decisions, including house buying, rent affordability, student loans, credit, borrowing, savings, and financial education.",
        ],
        // Government, elections, policy, diplomacy, defence, and public affairs.
        Category::Politics => &[
            "Politics and government: ministers, parliament, political parties, elected leaders, legislation, public administration, policy proposals, budgets, and political accountability.",
            "Democratic politics: elections, by-elections, campaigns, candidates, polling, voting intention, manifestos, party leadership, coalitions, and election results.",
            "International affairs and national security: diplomacy, treaties, sanctions, defence policy, armed forces, intelligence agencies, war, geopolitical threats, and foreign relations.",
            "Public policy debates about schools, migration, public services, infrastructure, online safety, regulation, civil liberties, or how government should respond.",
            "Political analysis and commentary about party strategy, ideology, public opinion, constitutional questions, devolution, independence, or relations between states.",
        ],
        // Crime, courts, policing, legal rights, enforcement, and public safety cases.
        Category::Law => &[
            "Law and justice reporting about courts, judges, trials, legal rulings, appeals, lawsuits, rights, liability, sentencing, prisons, or constitutional judgments.",
            "Crime and policing: murder, assault, sexual offences, abuse, fraud, theft, arrests, charges, criminal investigations, evidence, suspects, victims, or convictions.",
            "Regulatory and professional enforcement: unlawful conduct, official investigations, fines, bans, misconduct hearings, safeguarding breaches, or failures of legal duty.",
            "Serious local public safety incidents investigated by authorities, including unexplained deaths, missing people, fatal accidents, fires, inquests, emergency searches, or dangerous animals.",
            "Justice and protection for victims, children, or vulnerable people, including domestic abuse, exploitation, discrimination, negligence, and institutional accountability.",
        ],
        // Healthcare, disease, treatment, wellbeing, and medical systems.
        Category::Health => &[
            "Health and medicine: patients, doctors, hospitals, the NHS, clinics, diagnosis, treatment, surgery, prescriptions, care quality, and healthcare services.",
            "Disease and public health: infections, outbreaks, cancer, chronic illness, mental illness, addiction, mortality, vaccination, prevention, and population health.",
            "Medical research and therapy: clinical trials, pharmaceuticals, drug discovery, medical devices, treatment effectiveness, patient safety, and medical evidence.",
            "Mental health, wellbeing, nutrition, weight management, rehabilitation, physical activity, and medically grounded advice for improving health.",
            "Healthcare failures or safeguarding concerns involving patients, medical records, clinical decisions, hospital trusts, care providers, or health regulators.",
        ],
        // Entertainment, media, arts, celebrity, and cultural life.
        Category::Culture => &[
            "Culture and entertainment: films, television series, streaming programmes, books, theatre, art, music, concerts, festivals, performers, and reviews.",
            "Media and broadcasting: broadcasters, documentaries, television presenters, podcasts, journalism, audience reactions, programme schedules, and media controversies.",
            "Celebrities and public culture: actors, musicians, famous personalities, awards, popular franchises, fandom, celebrity news, and showbusiness.",
            "Social and cultural features about identity, public attitudes, heritage, cities, traditions, language, racism, or how people experience society.",
            "Entertainment recommendations and releases, including what to watch, streaming catalogues, trailers, cast announcements, rankings, and franchise merchandise.",
        ],
        // Home life, shopping, food, travel, style, and everyday personal interests.
        Category::Lifestyle => &[
            "Everyday lifestyle and the home: cooking, recipes, kitchens, cleaning, decorating, gardening, furniture, household upkeep, hosting, and domestic advice.",
            "Consumer shopping and deals for non-technical personal or household products, including mattresses, clothing, shoes, appliances, outdoor goods, sales, discounts, and buying guides.",
            "Food, drink, restaurants, holidays, travel experiences, hotels, leisure, festivals, hobbies, dating, family life, beauty, fashion, and personal style.",
            "Housing and living choices: homes, interiors, moving, renting, property lifestyle, neighbourhood life, and practical household improvements.",
            "Personal routines and advice about everyday comfort, recreation, relationships, productivity, or wellbeing where clinical medicine is not the subject.",
        ],
        // Vehicles, travel networks, public transport, and movement infrastructure.
        Category::Transport => &[
            "Transport and mobility: cars, electric vehicles, roads, motorways, traffic, driving, vehicle engineering, automotive manufacturers, and road safety.",
            "Railways and public transport: trains, stations, buses, metros, ticketing, passenger services, fares, delays, commuting, and rail infrastructure.",
            "Aviation and shipping: airlines, airports, aircraft, flights, ports, ferries, freight, shipping, logistics, and transport operators.",
            "Transport disruption or improvement involving vehicles or travel networks, such as road closures, vehicle crashes or fires, service suspensions, new routes, or passenger connectivity.",
            "Autonomous mobility and transport technology: self-driving cars, robotaxis, delivery vehicles, traffic systems, charging networks, or travel-focused drones.",
        ],
        // Environment, wildlife, climate, weather, conservation, and natural habitats.
        Category::Nature => &[
            "Nature and conservation: wildlife, animals, birds, fish, plants, forests, habitats, biodiversity, ecology, conservation groups, and protected land.",
            "Climate and weather: climate change, global warming, emissions, heatwaves, temperature records, flooding, drought, storms, wildfires, and extreme conditions.",
            "Environmental harm and restoration: pollution, sewage, waste, contaminated rivers, habitat destruction, rewilding, rainforest restoration, and ecosystem recovery.",
            "Energy and sustainability where the subject is environmental impact: renewables, net zero, green jobs, recycling, decarbonisation, and clean technology.",
            "The natural world and earth systems: oceans, rivers, geology, earthquakes, volcanoes, landscapes, environmental monitoring, and ecological research.",
        ],
        // Physical consumer electronics, computing devices, and hardware products.
        Category::Technology => &[
            "Consumer technology hardware: phones, tablets, laptops, televisions, cameras, headphones, speakers, smart home devices, and electronic gadgets.",
            "Hardware engineering and products: processors, chips, GPUs, screens, batteries, sensors, networking equipment, semiconductors, and device performance.",
            "Wearable and personal devices: smartwatches, fitness trackers, smart rings, augmented-reality glasses, virtual-reality headsets, accessories, and product reviews.",
            "Technology product news about device launches, specifications, comparisons, prototypes, hands-on testing, upgrades, pricing, or manufacturer announcements.",
            "Connected household electronics and physical digital products, such as security cameras, doorbells, televisions, routers, appliances, and audio equipment.",
        ],
        // Programs, online systems, privacy, cybersecurity, and developer work.
        Category::Software => &[
            "Software engineering and development: programming, source code, databases, APIs, developer tools, operating systems, open source projects, testing, and deployment.",
            "Cybersecurity and digital safety: hacking, malware, vulnerabilities, data breaches, ransomware, authentication, encryption, cyberattacks, and security research.",
            "Privacy and digital rights: tracking, personal data, surveillance, data retention, online identity, VPNs, platform privacy controls, and secure communications.",
            "Applications and internet services: apps, browsers, cloud platforms, productivity software, software updates, extensions, subscriptions, and digital features.",
            "Technical infrastructure and engineering practice: servers, cloud computing, containers, networks, GitHub projects, performance, reliability, and system administration.",
        ],
        // AI models, machine learning, agents, generation, and AI policy effects.
        Category::AI => &[
            "Artificial intelligence and machine learning: AI models, neural networks, language models, foundation models, training, inference, evaluation, and AI research.",
            "Generative AI products and assistants: ChatGPT, Claude, Gemini, copilots, chatbots, image generation, voice models, prompts, agents, and AI workflows.",
            "AI industry and model competition: OpenAI, Anthropic, DeepMind, DeepSeek, model releases, benchmarks, leaderboards, token pricing, capabilities, and open weights.",
            "AI impact and governance: automation, employment effects, AI safety, algorithmic harms, data centres for AI, public attitudes, copyright, policy, and regulation of AI.",
            "Uses of AI as the central subject, including autonomous agents, AI-assisted science, algorithmic decisions, machine-generated content, and AI-enabled products.",
        ],
        // Scientific research, space exploration, and discovery.
        Category::Science => &[
            "Science and research: experiments, researchers, laboratories, discoveries, evidence, scientific papers, universities, theories, and academic findings.",
            "Space and astronomy: NASA, astronauts, rockets, Moon missions, space stations, satellites, telescopes, planets, stars, galaxies, and spaceflight.",
            "Physics, chemistry, biology, genetics, evolution, archaeology, palaeontology, materials science, and fundamental scientific investigation.",
            "Exploration and scientific equipment developed to measure, observe, test, or operate in extreme environments including space or the deep ocean.",
        ],
        // Competitive sport, teams, athletes, competitions, and results.
        Category::Sports => &[
            "Sport and athletic competition: football, rugby, cricket, tennis, golf, athletics, cycling, boxing, racing, and other organised sports.",
            "Teams and athletes: players, managers, coaches, transfers, contracts, selection, injuries, training, club affairs, leagues, and sporting careers.",
            "Matches and tournaments: fixtures, results, scores, goals, championships, cups, tours, qualification, rankings, records, and title races.",
            "International and elite sport: Olympics, Paralympics, World Cups, Grand Slams, Six Nations, IPL, Premier League, Formula One, and major sporting events.",
            "Features about athletes or sport as lived experience, including competition finances, discrimination in sport, participation, performance, or sporting legacy.",
        ],
        // Video games, game studios, esports, releases, and gameplay.
        Category::Gaming => &[
            "Video games and interactive entertainment: game releases, gameplay, reviews, story campaigns, characters, levels, mechanics, updates, and downloadable content.",
            "Games industry and studios: game developers, publishers, consoles, PlayStation, Xbox, Nintendo, PC gaming, acquisitions, cancellations, and launch plans.",
            "Gaming communities and competition: esports, multiplayer games, live-service games, streamers, speedruns, mods, online shooters, and player communities.",
            "Video game hardware and accessories when discussed for playing games, including controllers, consoles, gaming PCs, handhelds, and virtual-reality games.",
        ],
    }
}
