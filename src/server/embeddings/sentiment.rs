pub fn labels(sentiment: &str) -> &'static [&'static str] {
    match sentiment {
        "positive" => &[
            "A story celebrating a successful achievement or award.",
            "A medical story reporting recovery or a treatment improving patients' lives.",
            "A rescue or safety effort that protected people from harm.",
            "A peace agreement or constructive cooperation resolving a dispute.",
            "A conservation project restoring habitats or protecting wildlife.",
            "A useful invention or scientific breakthrough providing clear benefits.",
            "A sporting victory or outstanding performance presented with admiration.",
            "An economic improvement creating jobs or helping households.",
            "A legal outcome protecting victims or correcting an injustice.",
            "A community donation or volunteer effort supporting people in need.",
            "A strongly favourable review describing satisfaction with a product or experience.",
            "A hopeful development offering progress after a difficult situation.",
        ],
        "negative" => &[
            "A tragic story about a death, fatal accident, or bereavement.",
            "A violent crime story involving assault, rape, abuse, or exploitation.",
            "A corruption or fraud scandal involving wrongdoing and conviction.",
            "A war story reporting attacks, escalation, destruction, or casualties.",
            "An environmental story reporting pollution or serious damage to wildlife.",
            "A public service failure leaving vulnerable people at risk.",
            "A business story about job losses, bankruptcy, or financial collapse.",
            "A political story marked by bitter conflict, condemnation, or backlash.",
            "A health story describing serious disease, unsafe conditions, or preventable harm.",
            "An accident or disaster causing injuries, evacuation, or severe disruption.",
            "A story reporting discrimination, human rights violations, or intimidation.",
            "A negative review describing a defective product or disappointing experience.",
        ],
        _ => &[],
    }
}
