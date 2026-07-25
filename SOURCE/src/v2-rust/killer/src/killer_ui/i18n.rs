//! **Internationalization runtime** — string localization with locale, fallback, and interpolation.

use std::collections::HashMap;

/// A locale bundle: key → translated string.
pub type LocaleBundle = HashMap<String, String>;

/// The i18n runtime: manages multiple locales and string resolution.
#[derive(Debug, Clone)]
pub struct I18n {
    /// Current active locale.
    pub locale: String,
    /// Fallback locale (used if key missing in active locale).
    pub fallback: String,
    /// Locale → bundle map.
    bundles: HashMap<String, LocaleBundle>,
    /// Missing key log for development.
    pub missing_keys: Vec<(String, String)>,
}

impl I18n {
    pub fn new(locale: &str, fallback: &str) -> Self {
        Self {
            locale: locale.to_string(),
            fallback: fallback.to_string(),
            bundles: HashMap::new(),
            missing_keys: Vec::new(),
        }
    }

    /// Register a locale bundle.
    pub fn add_bundle(&mut self, locale: &str, bundle: LocaleBundle) {
        self.bundles.insert(locale.to_string(), bundle);
    }

    /// Set active locale.
    pub fn set_locale(&mut self, locale: &str) {
        self.locale = locale.to_string();
    }

    /// Resolve a key. Tries active locale, then fallback, then returns the key itself.
    pub fn t(&mut self, key: &str) -> String {
        if let Some(bundle) = self.bundles.get(&self.locale) {
            if let Some(val) = bundle.get(key) { return val.clone(); }
        }
        if self.locale != self.fallback {
            if let Some(bundle) = self.bundles.get(&self.fallback) {
                if let Some(val) = bundle.get(key) { return val.clone(); }
            }
        }
        self.missing_keys.push((self.locale.clone(), key.to_string()));
        key.to_string()
    }

    /// Resolve with interpolation: `{name}` → value from params.
    pub fn t_with(&mut self, key: &str, params: &HashMap<String, String>) -> String {
        let mut result = self.t(key);
        for (k, v) in params {
            result = result.replace(&format!("{{{}}}", k), v);
        }
        result
    }

    /// Pluralization: picks key.zero / key.one / key.other based on count.
    pub fn t_plural(&mut self, key: &str, count: usize) -> String {
        let suffix = match count {
            0 => "zero",
            1 => "one",
            _ => "other",
        };
        let plural_key = format!("{}.{}", key, suffix);
        let result = self.t(&plural_key);
        // If the plural key wasn't found (returned the key itself), try base key
        if result == plural_key {
            self.t(key).replace("{count}", &count.to_string())
        } else {
            result.replace("{count}", &count.to_string())
        }
    }

    /// List available locales.
    pub fn available_locales(&self) -> Vec<&str> {
        self.bundles.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for I18n {
    fn default() -> Self { Self::new("en", "en") }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bundles() -> I18n {
        let mut i18n = I18n::new("en", "en");
        let mut en = LocaleBundle::new();
        en.insert("greeting".into(), "Hello".into());
        en.insert("welcome".into(), "Welcome, {name}!".into());
        en.insert("items.zero".into(), "No items".into());
        en.insert("items.one".into(), "1 item".into());
        en.insert("items.other".into(), "{count} items".into());
        i18n.add_bundle("en", en);

        let mut fr = LocaleBundle::new();
        fr.insert("greeting".into(), "Bonjour".into());
        fr.insert("welcome".into(), "Bienvenue, {name} !".into());
        i18n.add_bundle("fr", fr);
        i18n
    }

    #[test]
    fn basic_translation() {
        let mut i18n = sample_bundles();
        assert_eq!(i18n.t("greeting"), "Hello");
        i18n.set_locale("fr");
        assert_eq!(i18n.t("greeting"), "Bonjour");
    }

    #[test]
    fn fallback_locale() {
        let mut i18n = sample_bundles();
        i18n.set_locale("fr");
        // "items.one" not in fr, falls back to en
        assert_eq!(i18n.t("items.one"), "1 item");
    }

    #[test]
    fn interpolation() {
        let mut i18n = sample_bundles();
        let mut params = HashMap::new();
        params.insert("name".into(), "Alice".into());
        assert_eq!(i18n.t_with("welcome", &params), "Welcome, Alice!");
    }

    #[test]
    fn pluralization() {
        let mut i18n = sample_bundles();
        assert_eq!(i18n.t_plural("items", 0), "No items");
        assert_eq!(i18n.t_plural("items", 1), "1 item");
        assert_eq!(i18n.t_plural("items", 5), "5 items");
    }

    #[test]
    fn missing_key_tracked() {
        let mut i18n = sample_bundles();
        let _ = i18n.t("nonexistent");
        assert_eq!(i18n.missing_keys.len(), 1);
        assert_eq!(i18n.missing_keys[0].1, "nonexistent");
    }
}
