use crate::shared::Category;

pub const fn labels(category: Category) -> &'static [&'static str] {
    match category {
        // Companies, money, employment, markets, and the economy.
        Category::Business => &[
            "Business, companies, commerce, retail, manufacturing, suppliers, contracts, corporate strategy, profits, earnings, investment, ownership, mergers, acquisitions, bankruptcy, commercial deals, logistics, production, distribution, and supply chains.",
            "Finance and the economy: markets, stocks, shares, bonds, banking, lending, mortgages, pensions, insurance, inflation, interest rates, taxation, wages, trade, productivity, growth, exports, imports, prices, and market conditions.",
            "Employment and enterprise: jobs, hiring, salaries, redundancies, workplace policy, industrial relations, entrepreneurship, skills, careers, apprenticeships, placements, universities working with employers, and workforce development.",
        ],
        // Government, elections, policy, diplomacy, defence, and public affairs.
        Category::Politics => &[
            "Government, parliament, ministers, legislation, public administration, budgets, public services, civil service, planning officers, regulation, public sector staffing, policy decisions, housing policy, first-time buyers, interest-free loans, grants, public funds, schools policy, migration, and local government.",
            "Elections and political leadership: parties, candidates, polling, voting, manifestos, campaigns, debates, presidents, prime ministers, first ladies, White House, leadership contests, coalitions, devolution, independence, and public opinion.",
            "Foreign affairs and national security: diplomacy, defence policy, armed forces, intelligence agencies, GCHQ, espionage, hostile states, Russia, China, sanctions, treaties, war, infrastructure threats, democracy threats, and relations between states.",
        ],
        // Crime, courts, policing, legal rights, enforcement, and public safety cases.
        Category::Law => &[
            "Courts, justice, legal process, trials, judges, lawsuits, appeals, rulings, rights, liability, sentencing, prisons, constitutional judgments, compensation claims, environmental litigation, corporate liability, and government legal action.",
            "Crime and policing: murder, assault, sexual offences, abuse, fraud, theft, arrests, charges, suspects, evidence, convictions, criminal investigations, safeguarding breaches, victims, vulnerable people, exploitation, and institutional accountability.",
            "Public safety investigations involving people: unexplained deaths, missing people, bodies recovered from water, fatal accidents, dog attacks, fires, emergency searches, inquests, misconduct hearings, official investigations, bans, fines, and legal duty.",
        ],
        // Healthcare, disease, treatment, wellbeing, and medical systems.
        Category::Health => &[
            "Cancer, oncology, diagnosed, diagnosis, illness, treatment, chemotherapy, patient, medical condition, and public figure health.",
            "Health services and medicine: patients, doctors, hospitals, NHS, clinics, diagnosis, diagnosed with cancer, cancer diagnosis, public figures with illness, politicians or officials diagnosed with cancer, treatment, surgery, prescriptions, care quality, healthcare services, medical records, hospital trusts, care providers, and regulators.",
            "Disease and public health: cancer, oncology, infections, outbreaks, chronic illness, mental illness, addiction, mortality, vaccination, prevention, population health, diagnosis, treatment, patient safety, and safeguarding concerns.",
            "Medical research and wellbeing: clinical trials, pharmaceuticals, drug discovery, medical devices, treatment effectiveness, medical evidence, mental health, nutrition, rehabilitation, physical activity, and health advice.",
        ],
        // Entertainment, media, arts, celebrity, and cultural life.
        Category::Culture => &[
            "Entertainment and the arts: films, television, streaming, books, theatre, art, music, concerts, festivals, performers, reviews, trailers, cast announcements, awards, franchises, fandom, and showbusiness.",
            "Media and cultural life: broadcasters, documentaries, presenters, podcasts, journalism, media controversies, celebrities, famous personalities, identity, public attitudes, heritage, cities, traditions, language, racism, and society.",
            "Recommendations and releases: what to watch, streaming catalogues, online viewing guides, television previews, film rankings, crime thriller series, pop culture, merchandise, and leisure entertainment.",
        ],
        // Home life, shopping, food, travel, style, and everyday personal interests.
        Category::Lifestyle => &[
            "Home and everyday living: cooking, recipes, kitchens, cleaning, decorating, furniture, household upkeep, hosting, interiors, moving, renting, neighbourhood life, household improvements, utilities, energy bills, household bills, and cost of living.",
            "Consumer shopping and personal life: mattresses, clothing, shoes, appliances, outdoor goods, sales, discounts, buying guides, food, drink, restaurants, holidays, travel, hotels, leisure, hobbies, dating, family life, beauty, fashion, and style.",
            "Personal routines and advice: comfort, recreation, relationships, daily habits, practical tips, home comfort, wellness routines, and everyday service journalism.",
        ],
        // Vehicles, travel networks, public transport, and movement infrastructure.
        Category::Transport => &[
            "Road transport and vehicles: cars, electric vehicles, EVs, car launches, car design, automotive brands, Ferrari, driving, vehicle engineering, automotive manufacturers, road safety, crashes, vehicle fires, traffic, roads, and motorways.",
            "Public transport and travel networks: trains, stations, buses, metros, ticketing, passenger services, fares, delays, commuting, rail infrastructure, road closures, service suspensions, new routes, and passenger connectivity.",
            "Aviation, shipping, freight, and mobility technology: airlines, airports, aircraft, flights, ports, ferries, shipping, logistics, transport operators, self-driving cars, robotaxis, charging networks, delivery vehicles, and travel drones.",
        ],
        // Environment, wildlife, climate, weather, conservation, and natural habitats.
        Category::Nature => &[
            "Wildlife, plants, habitats, and conservation: animals, birds, fish, sharks, plants, tomatoes, gardening, growing, pruning, harvesting, forests, rainforest restoration, biodiversity, ecology, conservation groups, protected land, rewilding, and habitat recovery.",
            "Climate, weather, and environmental harm: climate change, global warming, emissions, heatwaves, temperature records, flooding, drought, storms, wildfires, pollution, sewage, waste, contaminated rivers, ecosystem damage, and sustainability.",
            "Earth systems and waterways: oceans, beaches, coasts, dead fish, washed up sea life, discarded catch, rivers, watercourses, landscapes, geology, earthquakes, volcanoes, environmental monitoring, ecological research, renewables, net zero, and recycling.",
        ],
        // Physical consumer electronics, computing devices, and hardware products.
        Category::Technology => &[
            "Consumer electronics and hardware: phones, tablets, laptops, televisions, physical cameras, headphones, speakers, routers, smart home devices, wearables, smartwatches, fitness trackers, smart rings, VR headsets, AR glasses, accessories, and gadgets.",
            "Hardware engineering and product coverage: processors, chips, GPUs, screens, batteries, sensors, networking equipment, semiconductors, device performance, specifications, comparisons, prototypes, hands-on testing, upgrades, pricing, and device manufacturer announcements.",
            "Connected household and personal devices: security cameras, doorbells, televisions, appliances, audio equipment, Wi-Fi, routers, home networking, battery life, device reviews, product launches, and consumer tech refreshes.",
        ],
        // Programs, online systems, privacy, cybersecurity, and developer work.
        Category::Software => &[
            "Programming and developer work: code, source code, databases, APIs, open source, testing, deployment, compilers, language runtimes, virtual machines, bytecode, register allocators, compiler internals, ZJIT, libraries, packages, frameworks, and build tools.",
            "Apps, editors, and internet services: Emacs, terminals, IDEs, plugins, browsers, cloud platforms, productivity software, camera apps, photo editors, iPhone apps, RAW processing, updates, extensions, subscriptions, digital features, and online systems.",
            "Cybersecurity and operations: hacking, malware, vulnerabilities, data breaches, ransomware, encryption, security research, servers, cloud computing, containers, networks, GitHub, performance, reliability, and system administration.",
        ],
        // AI models, machine learning, agents, generation, and AI policy effects.
        Category::AI => &[
            "Machine learning and models: neural networks, language models, foundation models, training, inference, evaluation, benchmarks, open weights, fine-tuning, model safety, model risk, training data, and AI research.",
            "Generative AI products and assistants: ChatGPT, Claude, Gemini, OpenAI, Anthropic, DeepMind, DeepSeek, copilots, chatbots, image generation, voice models, prompts, prompt engineering, AI productivity, AI workflow tricks, agents, autonomous assistants, and machine-generated content.",
            "AI industry and policy effects: model releases, leaderboards, token pricing, capabilities, assistant products, AI-assisted science, workplace automation, governance, safety debates, and commercial AI services.",
        ],
        // Scientific research, space exploration, and discovery.
        Category::Science => &[
            "Science and research: experiments, researchers, laboratories, discoveries, evidence, scientific papers, universities, theories, academic findings, physics, chemistry, biology, genetics, evolution, archaeology, palaeontology, and materials science.",
            "Space and astronomy: NASA, astronauts, payload scientists, rockets, Moon missions, space stations, satellites, telescopes, planets, stars, galaxies, spaceflight, space missions, crew announcements, and exploration.",
            "Scientific instruments and exploration: equipment developed to measure, observe, test, or operate in extreme environments, including space, deep ocean, laboratories, field research, major discoveries, and fundamental investigation.",
        ],
        // Competitive sport, teams, athletes, competitions, and results.
        Category::Sports => &[
            "Sport and athletic competition: football, rugby, cricket, tennis, golf, athletics, cycling, boxing, racing, organised sport, players, managers, coaches, transfers, contracts, selection, injuries, training, club affairs, leagues, and careers.",
            "Matches and tournaments: fixtures, results, scores, goals, championships, cups, tours, qualification, rankings, records, title races, Olympics, Paralympics, World Cups, Grand Slams, Six Nations, IPL, Premier League, Formula One, and elite events.",
            "Sports business and culture: team finances, discrimination, participation, performance, sporting legacy, fan discussion, match controversy, rows, handshake disputes, player comments, and tournament chatter.",
        ],
        // Video games, game studios, esports, releases, and gameplay.
        Category::Gaming => &[
            "Video games and interactive entertainment: game releases, gameplay, reviews, story campaigns, characters, levels, mechanics, updates, downloadable content, game deals, PlayStation, Xbox, Nintendo, PC gaming, and console games.",
            "Games industry and communities: developers, publishers, studios, acquisitions, cancellations, launch plans, esports, multiplayer games, live-service games, streamers, speedruns, mods, online shooters, and player communities.",
            "Gaming hardware and promotions: controllers, consoles, gaming PCs, handhelds, virtual-reality games, accessories, PS5 games, game discounts, limited-time deals, and promotional sales.",
        ],
    }
}
