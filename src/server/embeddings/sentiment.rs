pub fn labels(sentiment: &str) -> &'static [&'static str] {
    match sentiment {
        "positive" => &[
            "A story celebrating success, achievement, praise, or a clear win.",
            "A medical story about recovery, treatment success, or improved wellbeing.",
            "A rescue, safety effort, or protection from harm.",
            "A peace agreement, compromise, or constructive resolution of a dispute.",
            "A conservation or environmental project restoring habitats or protecting wildlife.",
            "A useful invention, scientific breakthrough, or practical improvement.",
            "A sporting victory or standout performance presented positively.",
            "An economic improvement, job creation, or benefit to households.",
            "A legal outcome correcting an injustice or protecting victims.",
            "A donation, volunteer effort, or community action helping other people.",
            "A favourable review, recommendation, or happy personal experience.",
            "A hopeful or encouraging development after a difficult period.",
        ],
        "negative" => &[
            "A tragic story about death, loss, bereavement, or serious injury.",
            "A violent crime story involving assault, rape, abuse, or exploitation.",
            "A corruption, fraud, or wrongdoing scandal.",
            "A war story with attacks, escalation, destruction, or casualties.",
            "An environmental story about pollution or serious damage to wildlife.",
            "A public service failure leaving vulnerable people at risk.",
            "A business story about job losses, collapse, or financial strain.",
            "A political story marked by conflict, condemnation, or backlash.",
            "A health story describing serious illness, unsafe conditions, or preventable harm.",
            "An accident or disaster causing injuries, evacuation, or disruption.",
            "A story about discrimination, rights violations, or intimidation.",
            "A negative review, complaint, or disappointing experience.",
            "A missing person, body found, or search-and-rescue story with a grim outcome.",
        ],
        _ => &[],
    }
}
