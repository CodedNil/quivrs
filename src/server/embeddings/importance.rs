pub fn labels(importance: &str) -> &'static [&'static str] {
    match importance {
        "important" => &[
            "National public significance: government decision, new law, court ruling, regulation, election outcome, or policy that changes rights or services.",
            "Large-scale harm or risk: public emergency, war, terrorism, epidemic, disaster, mass casualties, or national security warning.",
            "Systemic wrongdoing and accountability: criminal trial, conviction, inquiry, corruption case, institutional failure, safeguarding scandal, rights violation, or systemic injustice.",
            "Broad economic impact: national prices, tax changes, interest rates, investment, job losses, household bills, energy bills rise, energy bills, price cap, inflation, cost of living, and higher bills.",
            "Government housing help: first-time buyers, interest-free loans, housing fund, first homes fund, grants, public funds, housing policy, affordability crisis, and home ownership support.",
            "Essential services and infrastructure: healthcare, schools, water, communications, transport, housing, or digital security affecting communities.",
            "Major climate or environmental consequence: climate extremes, temperature records, river restoration, wildlife recovery, pollution incident, ecosystem damage, conservation action, or sustainability change.",
            "Significant medical or scientific development: serious health risk, treatment breakthrough, important research finding, cancer diagnosis, NHS trust failures, patient safety failure, space mission, or major discovery.",
            "Major public consequence affecting many people, a region, an industry, a nation, or international affairs.",
        ],
        "unimportant" => &[
            "Routine promotional content: sale, deal, discount, coupon, shopping recommendation, product roundup, mattress sale, last-chance sale, $400 off, or limited-time bargain.",
            "Routine consumer product coverage: review, comparison, teaser, minor update, gadget specification, appliance, mattress, wearable, phone, or accessory.",
            "Everyday lifestyle service content: cooking, cleaning, gardening, decorating, exercise tip, beauty, fashion, dating, holiday advice, home comfort, or personal routine.",
            "Light entertainment and leisure: streaming recommendations, release list, celebrity cameo, television preview, film ranking, game review, hobby feature, or pop-culture chatter.",
            "Routine sports coverage: transfer report, team selection, match preview, normal result, player comment, minor injury, fixture, or fan discussion.",
            "Small and transient update: software tutorial, developer blog post, AI benchmark score, model price comparison, consumer technology refresh, Emacs article, or company marketing.",
            "Limited political datapoint or commentary: opinion poll, candidate list, local result, speech reaction, party speculation, or day-to-day debate.",
            "Local incident, local death, animal story, beach wash-up, dog attack, fatal dog attack, XL Bully attack, killed teenager, local tragedy, inquest, or small local emergency with limited wider consequences.",
            "Streaming guide, series preview, watch online, entertainment recommendation, and free release.",
            "Emacs, register allocator, compiler internals, software blog, and developer tool article.",
            "Sports row, handshake dispute, tournament chatter, and match controversy.",
            "Games on sale, PS5 games on sale, console bargain, limited-time gaming deal, and promo discount.",
        ],
        _ => &[],
    }
}
