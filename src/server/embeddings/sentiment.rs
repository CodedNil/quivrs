pub fn labels(sentiment: &str) -> &'static [&'static str] {
    match sentiment {
        "positive" => &[
            "Strongly positive good news: success, victory, achievement, celebration, praise, award, breakthrough, improvement, progress, opportunity, benefit, relief, or welcome result.",
            "People flourish and feel supported: wellbeing, successful treatment, good health, care, fairness, dignity, freedom, comfort, confidence, happiness, or security.",
            "A clearly beneficial public outcome: prosperity, job creation, affordability, investment, better services, cooperation, opportunity, public benefit, or improved quality of life.",
            "Hopeful environmental or scientific progress: habitat restored, species thriving, clean solution developed, successful mission, discovery, innovation, sustainability, or healthy ecosystems.",
            "A warm and affirming human story about generosity, volunteers, community support, inclusion, reunion, courage, happiness, kindness, or overcoming adversity.",
            "Unambiguously favourable appraisal: excellent, best, recommended, impressive, delighted, enjoyable, exciting, valuable, successful, record-breaking, or outperforming rivals.",
            "Sport or culture framed as triumph and enjoyment: win, title, exceptional performance, acclaim, popular success, entertaining release, or celebrated career.",
            "An optimistic and joyful outcome: reunion, renewal, celebration, welcome, reassurance, satisfaction, gratitude, hope, harmony, delight, or a fresh start.",
        ],
        "negative" => &[
            "Death and tragic loss reported as news: a young person found dead, body recovered after welfare concerns, flowers and tributes laid, fatal incident, drowning, bereavement, grief, mourning, or devastating loss.",
            "Violence and abuse causing harm: murder, shooting, stabbing, rape, sexual assault, child abuse, domestic abuse, exploitation, cruelty, terror, or victims traumatised.",
            "Serious wrongdoing and injustice: corruption, fraud, theft, misconduct, negligence, cover-up, data breach, rights violation, discrimination, conviction, or betrayal of trust.",
            "Danger and disaster: war, attack, invasion, casualties, fire, explosion, crash, outbreak, emergency, pollution, destruction, evacuation, threat, or unsafe conditions.",
            "Institutions failing people: preventable death, ignored warnings, safeguarding failure, service collapse, medical error, official apology, public anger, scandal, or loss of confidence.",
            "Households made worse off by rising costs: energy bills increase, higher price cap, people pay more, unaffordable essentials, soaring prices, poverty, job losses, shortage, bankruptcy, cuts, or worsening inequality.",
            "Conflict and condemnation: backlash, outrage, accusations, bitter dispute, protest, warning, fear, crisis, risk, hostile action, or damaging political controversy.",
            "Clearly unfavourable experience or result: bad, worst, disappointing, failed, broken, suspended, banned, delayed, unsafe, harmful, alarming, regretted, or under threat.",
        ],
        _ => &[],
    }
}
