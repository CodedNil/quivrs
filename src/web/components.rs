use dioxus::prelude::*;

#[allow(dead_code)]
pub mod base16 {
    pub const BASE00: &str = "#24273a"; // Default Background
    pub const BASE01: &str = "#1e2030"; // Lighter Background
    pub const BASE02: &str = "#363a4f"; // Selection Background
    pub const BASE03: &str = "#494d64"; // Comments, Invisibles, Line Highlighting
    pub const BASE04: &str = "#5b6078"; // Dark Foreground
    pub const BASE05: &str = "#cad3f5"; // Default Foreground
    pub const BASE06: &str = "#f4dbd6"; // Light Foreground
    pub const BASE07: &str = "#b7bdf8"; // Lightest Foreground
    pub const BASE08: &str = "#ed8796"; // Red: Variables, XML Tags, Markup Link Text, Markup Lists, Diff Deleted
    pub const BASE09: &str = "#f5a97f"; // Orange: Integers, Boolean, Constants, XML Attributes, Markup Link Url
    pub const BASE0A: &str = "#eed49f"; // Yellow: Classes, Markup Bold, Search Text Background
    pub const BASE0B: &str = "#a6da95"; // Green: Strings, Inherited Class, Markup Code, Diff Inserted
    pub const BASE0C: &str = "#8bd5ca"; // Cyan: Support, Regular Expressions, Escape Characters, Markup Quotes
    pub const BASE0D: &str = "#8aadf4"; // Blue: Functions, Methods, Attribute IDs, Headings
    pub const BASE0E: &str = "#c6a0f6"; // Magenta: Keywords, Storage, Selector, Markup Italic, Diff Changed
    pub const BASE0F: &str = "#f0c6c6"; // Brown: Deprecated, Opening/Closing Embedded Language Tags, e.g. <?php ?>
}

#[component]
pub fn image_figure(url: String, caption: String, margin: String) -> Element {
    rsx! {
        figure { margin,
            img {
                src: "{url}",
                alt: "{caption}",
                width: "100%",
                border_radius: "0.375rem",
                display: "block",
            }
            figcaption {
                font_size: "0.7rem",
                color: base16::BASE03,
                margin_top: "0.375rem",
                text_align: "center",
                "{caption}"
            }
        }
    }
}
