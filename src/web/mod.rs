use dioxus::prelude::*;
pub fn app() -> Element {
    rsx! {
        document::Title { "Quivrs" }
        document::Meta { name: "darkreader-lock" }
        // document::Link { rel: "icon", href: asset!("/assets/icon.svg") }
        div {  }
    }
}
