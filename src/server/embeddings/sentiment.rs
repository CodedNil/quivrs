pub fn labels(sentiment: &str) -> &'static [&'static str] {
    match sentiment {
        "positive" => &[
            "Socioeconomic progress and structural enhancement: major public optimization initiatives, expansion of state infrastructure, enhanced educational grants, institutional academic funding, robust state financial subsidies.",
            "Academic development and career enrichment: multi-disciplinary undergraduate advancement, vocational real-world placements, institutional student training programs, meaningful employment readiness paths.",
            "Commercial prosperity, excellent product discounts, and consumer retail advantages: massive holiday weekend bargain sales, spectacular e-commerce price drops, multi-percentage retail reductions, fantastic shopping deal events, clearance savings, promotional markdowns on top-rated home appliances, mattresses, footwear, televisions, and tablets.",
            "Diplomatic harmony and social cohesion: successful peaceful mediation, cross-border cooperation treaties, community solidarity, regional stabilization, civic alignment, international security partnerships.",
            "Recreational joy and cultural health: domestic comfort, positive community morale, creative restorations, cultural heritage preservation, celebratory public milestones, local leisure enhancements, gardening and outdoor backyard recreation, culinary lifestyle tips.",
            "Technical innovation and operational breakthroughs: next-generation computational architectures, quantum processing achievements, pioneering engineering breakthroughs, space exploration milestones, payload scientist selections, major collaborative scientific discoveries.",
            "Ecological revitalization: active wildlife preservation, habitat conservation successes, pollution remediation, successful ecosystem recovery, river system tracking initiatives, regional land preservation.",
        ],
        "negative" => &[
            "Sovereign warfare and national security emergencies: active military targeting, state-sponsored cyber sabotage, hostile foreign subversion tactics, critical infrastructure threats, espionage intelligence warnings, cross-border military strikes, imminent security breaches, wartime economic casualties, sudden energy price shocks.",
            "Macro-economic distress and consumer inflation: escalating cost-of-living crises, soaring household utility bills, regulatory pricing penalties, systemic supply chain collapses, global trade conflicts, punitive import tariffs.",
            "Institutional failures and administrative misconduct: public trust betrayals, corporate negligence scandals, systemic medical care failures, ignoring critical safety complaints, regulatory misconduct, forced administrative resignations.",
            "Extremist behavior and political radicalism: illicit partisan symbols, offensive extremist manifestations, public official misconduct controversies, intense community backlash over unethical records.",
            "Severe public tragedies, devastating loss of life, and fatal accidents: mass casualty events, lethal school building blazes, horrific structural fires, fatal injuries, ongoing body recovery rescue operations, accidental drowning disasters, sudden family bereavement crises.",
            "Violent crimes and predatory offences: criminal physical assaults, severe public safety violations, localized sexual felonies, felony law enforcement arrests.",
            "Pathological crises, advanced malignancies, and medical suffering: terminal physical diseases, devastating cancer diagnoses, severe oncological illnesses, life-threatening physiological failures, acute clinical distress, intense psychological suffering.",
            "Environmental devastation and ecological decay: hazardous chemical contamination, mass marine life mortality, toxic pollution spills, record-breaking global warmth anomalies, catastrophic planetary heat domes, illegal wildlife targeting, shooting of protected birds.",
            "Product design friction and functional degradation: highly polarizing product aesthetics, severe device operational defects, rapid battery degradation concerns, intense consumer dissatisfaction.",
            "Interpersonal discord and competitive hostility: professional handshake avoidance disputes, hostile athletic arguments, public legal rows, tense workplace altercations.",
        ],
        _ => &[],
    }
}
