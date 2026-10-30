//! User-facing strings, loaded from the Fluent files in `locales/`.
//!
//! The `.ftl` files are embedded at compile time, so the app ships as a single
//! executable. Adding a language means adding a `locales/<code>/fitil.ftl` file
//! and a variant to [`Language`].

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use unic_langid::LanguageIdentifier;

/// A UI language that ships with Fitil.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Turkish,
    English,
}

impl Language {
    pub const ALL: [Language; 2] = [Language::Turkish, Language::English];

    /// The operating system's language if Fitil supports it, otherwise English.
    pub fn from_system() -> Self {
        sys_locale::get_locale()
            .and_then(|locale| Self::from_locale(&locale))
            .unwrap_or(Language::English)
    }

    /// Matches a locale such as `tr-TR` or `en_US` by its primary language.
    fn from_locale(locale: &str) -> Option<Self> {
        let primary = locale.split(['-', '_']).next()?.to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|language| language.code() == primary)
    }

    pub fn code(self) -> &'static str {
        match self {
            Language::Turkish => "tr",
            Language::English => "en",
        }
    }

    /// The Fluent key holding this language's own name, e.g. "Türkçe".
    pub fn name_key(self) -> &'static str {
        match self {
            Language::Turkish => "language-tr",
            Language::English => "language-en",
        }
    }

    fn source(self) -> &'static str {
        match self {
            Language::Turkish => include_str!("../../../locales/tr/fitil.ftl"),
            Language::English => include_str!("../../../locales/en/fitil.ftl"),
        }
    }
}

/// All translated strings for one language.
pub struct Translations {
    language: Language,
    bundle: FluentBundle<FluentResource>,
}

impl Translations {
    pub fn new(language: Language) -> Self {
        let id: LanguageIdentifier = language
            .code()
            .parse()
            .expect("language codes are valid identifiers");
        let resource = FluentResource::try_new(language.source().to_owned())
            .unwrap_or_else(|(_, errors)| panic!("invalid {}.ftl: {errors:?}", language.code()));

        let mut bundle = FluentBundle::new(vec![id]);
        // Unicode isolation marks around arguments render as stray glyphs in the UI.
        bundle.set_use_isolating(false);
        bundle
            .add_resource(resource)
            .expect("bundled .ftl files have no duplicate keys");

        Self { language, bundle }
    }

    pub fn language(&self) -> Language {
        self.language
    }

    /// The string for `key`, or the key itself if it is missing.
    pub fn get(&self, key: &str) -> String {
        self.format(key, None)
    }

    /// Like [`Translations::get`], filling in `{ $name }` placeholders from `args`.
    pub fn get_with(&self, key: &str, args: &FluentArgs) -> String {
        self.format(key, Some(args))
    }

    fn format(&self, key: &str, args: Option<&FluentArgs>) -> String {
        let Some(pattern) = self
            .bundle
            .get_message(key)
            .and_then(|message| message.value())
        else {
            return key.to_owned();
        };
        let mut errors = Vec::new();
        self.bundle
            .format_pattern(pattern, args, &mut errors)
            .into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Message keys defined in a `.ftl` source (lines like `key = value`).
    fn keys(source: &str) -> Vec<&str> {
        let mut keys: Vec<&str> = source
            .lines()
            .filter(|line| line.starts_with(|c: char| c.is_ascii_alphabetic()))
            .filter_map(|line| line.split_once('=').map(|(key, _)| key.trim()))
            .collect();
        keys.sort_unstable();
        keys
    }

    #[test]
    fn every_language_defines_the_same_keys() {
        let english = keys(Language::English.source());
        for language in Language::ALL {
            assert_eq!(
                keys(language.source()),
                english,
                "{}.ftl must define exactly the keys of en.ftl",
                language.code()
            );
        }
    }

    #[test]
    fn every_language_loads_and_names_itself() {
        for language in Language::ALL {
            let translations = Translations::new(language);
            assert_ne!(translations.get(language.name_key()), language.name_key());
        }
    }

    #[test]
    fn placeholders_are_filled_in() {
        let mut args = FluentArgs::new();
        args.set("version", "1.2.3");
        let text = Translations::new(Language::Turkish).get_with("app-version", &args);
        assert_eq!(text, "Sürüm 1.2.3");
    }

    #[test]
    fn locales_match_by_primary_language() {
        assert_eq!(Language::from_locale("tr-TR"), Some(Language::Turkish));
        assert_eq!(Language::from_locale("en_US"), Some(Language::English));
        assert_eq!(Language::from_locale("EN"), Some(Language::English));
        assert_eq!(Language::from_locale("de-DE"), None);
    }
}
