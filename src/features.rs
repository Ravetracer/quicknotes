//! Optional feature areas that can be switched off in the preferences.
//!
//! A switched-off feature behaves as if it did not exist: its menu entries,
//! toolbar buttons and help sections are hidden, and its documents are
//! treated as what they are underneath (presentations and Kanban boards as
//! Markdown, chart blocks as code, PlantUML files as text). Nothing is
//! changed in the documents themselves.
//!
//! New installations start with all of them off; settings saved before the
//! switches existed get all of them on.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feature {
    Charts,
    Presentations,
    Kanban,
    Geocaching,
    PlantUml,
    DevTools,
    Flashcards,
}

/// The features with their preference labels and descriptions.
pub const LIST: &[(Feature, &str, &str)] = &[
    (Feature::Charts, "Charts", "Charts from Markdown tables (```chart blocks)"),
    (Feature::Presentations, "Presentations", "Slides from Markdown, presenter view and export"),
    (Feature::Kanban, "Kanban Boards", "Boards with columns, swimlanes and cards"),
    (Feature::Geocaching, "Geocaching Tools", "Ciphers, coordinates and puzzle helpers in the context menu"),
    (Feature::PlantUml, "PlantUML", "Diagram preview for PlantUML files (needs PlantUML installed)"),
    (
        Feature::DevTools,
        "Developer Tools",
        "Format, convert and encode JSON, XML, YAML, SQL, MongoDB and PHP arrays; line and case tools",
    ),
    (Feature::Flashcards, "Flash Cards", "Learning with flash cards and spaced repetition"),
];

impl Feature {
    fn bit(self) -> u8 {
        match self {
            Self::Charts => 1,
            Self::Presentations => 2,
            Self::Kanban => 4,
            Self::Geocaching => 8,
            Self::PlantUml => 16,
            Self::DevTools => 32,
            Self::Flashcards => 64,
        }
    }

    /// Id of the help section that describes the feature (also used by the
    /// help's `<!-- if … -->` markers).
    pub fn key(self) -> &'static str {
        match self {
            Self::Charts => "charts",
            Self::Presentations => "slides",
            Self::Kanban => "kanban",
            Self::Geocaching => "geocaching",
            Self::PlantUml => "plantuml",
            Self::DevTools => "devtools",
            Self::Flashcards => "flashcards",
        }
    }
}

/// Which features are on (stored in the settings).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(default)]
pub struct Features {
    pub charts: bool,
    pub presentations: bool,
    pub kanban: bool,
    pub geocaching: bool,
    pub plantuml: bool,
    /// Added after the other switches: on for settings that lack it.
    #[serde(default = "yes")]
    pub devtools: bool,
    #[serde(default = "yes")]
    pub flashcards: bool,
}

fn yes() -> bool {
    true
}

impl Features {
    /// All features on (settings from before the switches existed).
    pub fn all() -> Self {
        Self { charts: true, presentations: true, kanban: true, geocaching: true, plantuml: true, devtools: true, flashcards: true }
    }

    pub fn get(&self, f: Feature) -> bool {
        match f {
            Feature::Charts => self.charts,
            Feature::Presentations => self.presentations,
            Feature::Kanban => self.kanban,
            Feature::Geocaching => self.geocaching,
            Feature::PlantUml => self.plantuml,
            Feature::DevTools => self.devtools,
            Feature::Flashcards => self.flashcards,
        }
    }

    pub fn set(&mut self, f: Feature, on: bool) {
        match f {
            Feature::Charts => self.charts = on,
            Feature::Presentations => self.presentations = on,
            Feature::Kanban => self.kanban = on,
            Feature::Geocaching => self.geocaching = on,
            Feature::PlantUml => self.plantuml = on,
            Feature::DevTools => self.devtools = on,
            Feature::Flashcards => self.flashcards = on,
        }
    }

    fn bits(&self) -> u8 {
        LIST.iter().filter(|(f, ..)| self.get(*f)).map(|(f, ..)| f.bit()).sum()
    }

    fn from_bits(bits: u8) -> Self {
        let mut out = Self::default();
        for (f, ..) in LIST {
            out.set(*f, bits & f.bit() != 0);
        }
        out
    }
}

/// The features in use; all on until the settings are applied (and in tests).
static CURRENT: AtomicU8 = AtomicU8::new(u8::MAX);

pub fn apply(features: Features) {
    CURRENT.store(features.bits(), Ordering::Relaxed);
}

pub fn current() -> Features {
    Features::from_bits(CURRENT.load(Ordering::Relaxed))
}

pub fn on(f: Feature) -> bool {
    CURRENT.load(Ordering::Relaxed) & f.bit() != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_round_trip() {
        let mut f = Features::default();
        f.set(Feature::Kanban, true);
        f.set(Feature::PlantUml, true);
        assert_eq!(Features::from_bits(f.bits()), f);
        assert_eq!(Features::from_bits(Features::all().bits()), Features::all());
    }

    #[test]
    fn new_installs_start_minimal_old_settings_keep_everything() {
        assert_eq!(crate::store::Settings::default().features, Features::default());
        let old: crate::store::Settings = serde_json::from_str("{\"font_size\": 14.0}").unwrap();
        assert_eq!(old.features, Features::all());
        let partial: crate::store::Settings = serde_json::from_str("{\"features\": {\"charts\": true}}").unwrap();
        assert!(partial.features.charts && !partial.features.kanban);
        // Switches added later stay on for settings saved before them.
        assert!(partial.features.devtools);
        assert!(!crate::store::Settings::default().features.devtools);
    }
}
