pub fn labels(importance: &str) -> &'static [&'static str] {
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
