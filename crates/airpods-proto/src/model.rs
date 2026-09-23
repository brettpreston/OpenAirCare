//! Model-number based capability table (ported from the Android app's
//! `data/AirPods.kt`). Only the AirPods Pro 2 (Lightning and USB-C) and
//! AirPods Pro 3 expose the hearing-aid feature.

/// Model numbers that support the Hearing Aid feature.
pub const HEARING_AID_MODELS: &[&str] = &[
    // AirPods Pro 2 (Lightning)
    "A2931", "A2699", "A2698", // AirPods Pro 2 (USB-C)
    "A3047", "A3048", "A3049", // AirPods Pro 3
    "A3063", "A3064", "A3065",
];

pub fn supports_hearing_aid(model_number: &str) -> bool {
    let m = model_number.trim();
    HEARING_AID_MODELS.iter().any(|x| x.eq_ignore_ascii_case(m))
}

/// Friendly model family name for display.
pub fn family_name(model_number: &str) -> &'static str {
    match model_number.trim() {
        "A2931" | "A2699" | "A2698" => "AirPods Pro 2",
        "A3047" | "A3048" | "A3049" => "AirPods Pro 2 (USB-C)",
        "A3063" | "A3064" | "A3065" => "AirPods Pro 3",
        _ => "AirPods",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_lookup() {
        assert!(supports_hearing_aid("A3048"));
        assert!(supports_hearing_aid(" a2931 "));
        assert!(!supports_hearing_aid("A2084")); // AirPods 3
        assert_eq!(family_name("A3064"), "AirPods Pro 3");
    }
}
