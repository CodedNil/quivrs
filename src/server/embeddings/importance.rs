pub fn labels(importance: &str) -> &'static [&'static str] {
    match importance {
        "important" => &[
            "Macroeconomic state directives and fiscal legislation: statutory national economic systems, country-wide housing financial initiatives, state-backed financial subsidy programs, binding multi-million regulatory price caps, mandatory utility adjustments, major global trade realignments.",
            "Sovereign political governance and legislative changes: constitutional law amendments, state executive appointments, parliamentary legislation updates, national civil service systemic re-organizations.",
            "Geopolitical intelligence strategy and transnational defense: high-level intelligence agency directives, sovereign state security alerts, cross-border defense alliances, national democratic protection strategies, macro-threat assessments.",
            "Widespread societal transformations and demographics: country-wide census shifts, generational birth rate fluctuations, macro demographic fertility trends, broad public safety crises, extensive public welfare vulnerabilities.",
            "Generational scientific and technical achievements: foundational technological breakthroughs, global quantum computing research access, international scientific collaboration programs, paradigm-shifting medical discoveries.",
            "Severe national emergencies and massive infrastructure disasters: mass-fatality structural tragedies, lethal multi-casualty disasters, critical cross-border energy infrastructure updates, country-wide environmental litigation prosecutions.",
            "Macro ecological preservation: extensive statutory conservation efforts, sweeping river system safety mandates, high-level wildlife protections.",
        ],
        "unimportant" => &[
            "Routine consumer lifestyle and retail commerce: personal shopping guides, household appliance discounts, commercial furniture promotions, general electronics product updates, consumer car rumors, localized retail shopping events, personal culinary trends, backyard grilling guides, tomato gardening tips.",
            "Niche technical walkthroughs, internal software configurations, and low-level programming code adjustments: localized digital setups, code compiler optimizations, personal development workspace tools, isolated runtime programming tutorials, software documentation extracts, prompt optimization strategies, individual computer networking advice, software engine register allocation, method inlining details, localized router signal solutions.",
            "Localized community incidents and municipal occurrences: regional neighborhood disputes, local council administrative arguments, municipal park code updates, regional leisure destination news.",
            "Individual personal health updates: single-person medical diagnoses, isolated clinical treatments, personal health recovery profiles, individual advisory group nominations.",
            "Minor pop-culture entertainment and streaming media: commercial television broadcast schedules, streaming media series reviews, free online video viewing options, celebrity pop-culture reporting.",
            "Localized sports friction and field altercations: competitive athletic handshake arguments, grand slam referee disputes, single-match scoreboards, regional tournament formatting.",
            "Isolated local criminal incidents and singular offences: localized felony arrests, single-incident park accidents, localized beach law enforcement interventions, municipal anti-social behavior warnings.",
            "Minor regional conservation activities and local park upkeep: regional public access maintenance, localized volunteer community cleanups, municipal landmark preservation efforts, minor regional wildlife tracking, localized pollution surveying.",
            "Tactical combat updates, defense site liabilities, and local border actions: field military maneuvers, specific local evacuation bulletins, localized territorial artillery strikes, active tactical field deployments, regional corporate environmental liability cases.",
        ],
        _ => &[],
    }
}
