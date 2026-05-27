use crate::shared::Category;

pub const fn labels(category: Category) -> &'static [&'static str] {
    match category {
        // Companies, money, employment, markets, and the economy.
        Category::Business => &[
            "Companies, profits, earnings, investment, ownership, mergers, acquisitions, bankruptcy, retail, manufacturing, suppliers, and corporate strategy.",
            "Financial markets and services: stocks, shares, bonds, banking, lending, mortgages, pensions, insurance, fintech, cryptocurrency, and IPOs.",
            "Macroeconomics and commerce: inflation, interest rates, taxation, wages, trade, productivity, growth, export, import, and market conditions.",
            "Employment and enterprise: jobs, salaries, redundancies, industrial relations, entrepreneurship, workplace policy, skills, careers, and hiring.",
            "Business operations and supply chains: factories, production, logistics, wholesale, distribution, sourcing, contracts, and commercial deals.",
        ],
        // Government, elections, policy, diplomacy, defence, and public affairs.
        Category::Politics => &[
            "Government and parliament: ministers, parliament, legislation, public administration, budgets, policy, and accountability.",
            "Elections and campaigns: parties, candidates, polling, voting, manifestos, leadership contests, coalitions, and results.",
            "Foreign affairs and national security: diplomacy, treaties, sanctions, defence policy, armed forces, intelligence, war, and foreign relations.",
            "Public policy: schools, migration, housing policy, first-time buyers, loans, grants, public services, local government, civil service, planning officers, council officers, public sector staffing, infrastructure, regulation, and government decisions.",
            "Political analysis: strategy, ideology, public opinion, constitutional questions, devolution, independence, and relations between states.",
        ],
        // Crime, courts, policing, legal rights, enforcement, and public safety cases.
        Category::Law => &[
            "Courts and justice: judges, trials, legal rulings, appeals, lawsuits, rights, liability, sentencing, prisons, and constitutional judgments.",
            "Crime and policing: murder, assault, sexual offences, abuse, fraud, theft, arrests, charges, criminal investigations, evidence, suspects, and convictions.",
            "Regulatory enforcement: unlawful conduct, official investigations, fines, bans, misconduct hearings, safeguarding breaches, and legal duty.",
            "Inquests and public safety incidents involving people: unexplained deaths, missing people, fatal accidents, fires, emergency searches, and official investigations.",
            "Justice for victims and vulnerable people: domestic abuse, exploitation, discrimination, negligence, and institutional accountability.",
        ],
        // Healthcare, disease, treatment, wellbeing, and medical systems.
        Category::Health => &[
            "Health and medicine: patients, doctors, hospitals, NHS, clinics, diagnosis, treatment, surgery, prescriptions, care quality, and healthcare services.",
            "Disease and public health: cancer, diagnosed with cancer, cancer diagnosis, oncology, diagnosis, treatment, patients, doctors, infections, outbreaks, chronic illness, mental illness, addiction, mortality, vaccination, prevention, and population health.",
            "Medical research and therapy: clinical trials, pharmaceuticals, drug discovery, medical devices, treatment effectiveness, patient safety, and medical evidence.",
            "Mental health, wellbeing, nutrition, weight management, rehabilitation, physical activity, and health advice.",
            "Healthcare failures and safeguarding concerns involving patients, medical records, clinical decisions, hospital trusts, care providers, or regulators.",
        ],
        // Entertainment, media, arts, celebrity, and cultural life.
        Category::Culture => &[
            "Films, television, streaming, books, theatre, art, music, concerts, festivals, performers, and reviews.",
            "Media and broadcasting: broadcasters, documentaries, presenters, podcasts, journalism, schedules, and media controversies.",
            "Celebrities and showbusiness: actors, musicians, famous personalities, awards, franchises, fandom, and celebrity news.",
            "Social and cultural features: identity, public attitudes, heritage, cities, traditions, language, racism, and society.",
            "Entertainment recommendations and releases: what to watch, streaming catalogues, trailers, cast announcements, rankings, crime thriller series, and merchandise.",
        ],
        // Home life, shopping, food, travel, style, and everyday personal interests.
        Category::Lifestyle => &[
            "Home and lifestyle: cooking, recipes, kitchens, cleaning, decorating, gardening, furniture, household upkeep, hosting, and domestic advice.",
            "Consumer shopping and deals: mattresses, clothing, shoes, appliances, outdoor goods, sales, discounts, and buying guides.",
            "Food, drink, restaurants, holidays, travel, hotels, leisure, festivals, hobbies, dating, family life, beauty, fashion, and style.",
            "Everyday living: homes, interiors, moving, renting, neighbourhood life, household improvements, utilities, energy bills, household bills, and cost of living.",
            "Personal routines: comfort, recreation, relationships, productivity, and everyday advice.",
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
            "Wildlife and conservation: animals, birds, fish, plants, forests, habitats, biodiversity, ecology, conservation groups, and protected land.",
            "Climate and weather: climate change, global warming, emissions, heatwaves, temperature records, flooding, drought, storms, wildfires, and extreme conditions.",
            "Environmental harm and restoration: pollution, sewage, waste, contaminated rivers, habitat destruction, rewilding, rainforest restoration, and ecosystem recovery.",
            "Sustainability: renewables, net zero, green jobs, recycling, decarbonisation, and clean technology.",
            "Earth systems: oceans, rivers, geology, earthquakes, volcanoes, landscapes, environmental monitoring, and ecological research.",
        ],
        // Physical consumer electronics, computing devices, and hardware products.
        Category::Technology => &[
            "Consumer hardware: phones, tablets, laptops, televisions, cameras, headphones, speakers, smart home devices, and electronic gadgets.",
            "Hardware engineering: processors, chips, GPUs, screens, batteries, sensors, networking equipment, semiconductors, and device performance.",
            "Wearables and personal devices: smartwatches, fitness trackers, smart rings, augmented-reality glasses, virtual-reality headsets, accessories, and reviews.",
            "Device launches, specifications, comparisons, prototypes, hands-on testing, upgrades, pricing, and manufacturer announcements.",
            "Connected household electronics: security cameras, doorbells, televisions, routers, appliances, and audio equipment.",
        ],
        // Programs, online systems, privacy, cybersecurity, and developer work.
        Category::Software => &[
            "Programming and development: code, source code, databases, APIs, open source, testing, deployment, compilers, language runtimes, virtual machines, bytecode, register allocators, compiler internals, and ZJIT.",
            "Editors and developer tools: Emacs, terminals, libraries, packages, frameworks, IDEs, plugins, build tools, JITs, compiler toolchains, and bytecode interpreters.",
            "Cybersecurity and digital safety: hacking, malware, vulnerabilities, data breaches, ransomware, encryption, and security research.",
            "Apps and internet services: apps, browsers, cloud platforms, productivity software, updates, extensions, subscriptions, and digital features.",
            "Infrastructure and system administration: servers, cloud computing, containers, networks, GitHub, performance, reliability, and operations.",
        ],
        // AI models, machine learning, agents, generation, and AI policy effects.
        Category::AI => &[
            "Machine learning and neural networks: language models, foundation models, training, inference, evaluation, and research.",
            "Generative models and assistants: ChatGPT, Claude, Gemini, copilots, chatbots, image generation, voice models, prompts, and agents.",
            "Model releases and benchmarks: OpenAI, Anthropic, DeepMind, DeepSeek, leaderboards, token pricing, capabilities, and open weights.",
            "Model safety and training: model risk, training data, benchmarks, open weights, and fine-tuning.",
            "Chatbots and language-model products: machine-generated content, autonomous agents, AI-assisted science, and assistant products.",
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
            "Sport and athletic competition: football, rugby, cricket, tennis, golf, athletics, cycling, boxing, racing, and organised sport.",
            "Teams and athletes: players, managers, coaches, transfers, contracts, selection, injuries, training, club affairs, leagues, and careers.",
            "Matches and tournaments: fixtures, results, scores, goals, championships, cups, tours, qualification, rankings, records, and title races.",
            "International and elite sport: Olympics, Paralympics, World Cups, Grand Slams, Six Nations, IPL, Premier League, Formula One, and major sporting events.",
            "Sport features: finances, discrimination, participation, performance, and sporting legacy.",
        ],
        // Video games, game studios, esports, releases, and gameplay.
        Category::Gaming => &[
            "Video games and interactive entertainment: game releases, gameplay, reviews, story campaigns, characters, levels, mechanics, updates, downloadable content, and game deals.",
            "Games industry and studios: developers, publishers, consoles, PlayStation, Xbox, Nintendo, PC gaming, acquisitions, cancellations, and launch plans.",
            "Gaming communities and competition: esports, multiplayer games, live-service games, streamers, speedruns, mods, online shooters, and player communities.",
            "Video game hardware and accessories: controllers, consoles, gaming PCs, handhelds, virtual-reality games, and game discounts.",
        ],
    }
}
