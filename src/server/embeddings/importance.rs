pub fn labels(importance: &str) -> &'static [&'static str] {
    match importance {
        "important" => &[
            "High public importance and lasting consequence: a major government decision, new law, court ruling, regulation, constitutional issue, election outcome, or policy that changes people's rights or services.",
            "Severe threat to life, safety, or security: deaths, serious violence, abuse, public emergency, war, terrorism, military escalation, epidemic, disaster, or a credible national security warning.",
            "Accountability for grave wrongdoing: major criminal trial, conviction, inquiry, corruption case, institutional failure, safeguarding scandal, civil rights violation, or systemic injustice.",
            "Broad economic impact: national prices, energy bills, tax changes, interest rates, major investment or collapse, employment shock, household finances, or disruption affecting many people.",
            "Major public infrastructure or essential service consequence: healthcare, schools, water, energy, communications, transport networks, housing, or digital security affecting communities.",
            "Substantial environmental consequence: climate extremes or records, large pollution incident, ecosystem damage, environmental policy, conservation action, or major sustainability change.",
            "Significant medical or scientific development: serious health risk, treatment breakthrough, important research finding, space mission, major discovery, or technology with societal effects.",
            "A development worthy of sustained news attention because it affects vulnerable people, an entire locality, a large institution, an industry, a nation, or international affairs.",
        ],
        "unimportant" => &[
            "Low public importance and mainly promotional content: sale, deal, discount, coupon, shopping recommendation, product roundup, affiliate buying guide, sponsored feature, or limited-time bargain.",
            "Routine consumer product coverage: review, hands-on impression, comparison, rumour, launch teaser, minor update, gadget specification, appliance, mattress, wearable, phone, or accessory.",
            "Everyday lifestyle service content: cooking, cleaning, gardening, decorating, exercise tip, beauty, fashion, dating, holiday advice, home comfort, or personal routine.",
            "Light entertainment and leisure: streaming recommendations, release list, celebrity cameo, television preview, film ranking, collectible, game review, hobby feature, or pop-culture chatter.",
            "Routine sports coverage without wider consequences: transfer report, team selection, match preview, normal result, player comment, minor injury, fixture, or fan discussion.",
            "Small and transient update: incremental app feature, software tutorial, developer blog post, AI benchmark score, model price comparison, consumer technology refresh, or company marketing.",
            "Limited political datapoint or commentary: ordinary opinion poll, candidate list, local result, speech reaction, party speculation, webinar, survey promotion, or day-to-day Westminster debate.",
            "Personal interest or niche information without meaningful effects on public safety, rights, services, livelihoods, the environment, or a substantial number of people.",
        ],
        _ => &[],
    }
}
