use once_cell::sync::OnceCell;
use rust_embed::RustEmbed;
use std::collections::HashMap;

#[derive(RustEmbed)]
#[folder = "locales/"]
struct Asset;

type Translations = HashMap<String, String>;

static TRANSLATIONS: OnceCell<Translations> = OnceCell::new();

/// Initializes the translation system with the given language.
/// Defaults to "en" if the specified language is not found or is empty.
pub fn init(language: &str) {
    let lang_code = if language.is_empty() { "en" } else { language };

    let translations = load_translations(lang_code).unwrap_or_else(|| {
        if lang_code != "en" {
            // Fallback to English if the specified language file is not found
            load_translations("en").expect("Failed to load default English translations.")
        } else {
            panic!("Failed to load default English translations.");
        }
    });

    TRANSLATIONS.set(translations).expect("Failed to set translations");
}

/// Gets the translated string for a given key.
///
/// # Panics
///
/// Panics if the translation system has not been initialized with `init()`.
/// Panics if the key is not found in the loaded translations.
pub fn t(key: &str) -> String {
    let translations = TRANSLATIONS.get().expect("I18n system not initialized. Call init() first.");
    translations.get(key).cloned().unwrap_or_else(|| panic!("Translation key not found: {}", key))
}

/// Formats a translated string with the given arguments.
/// It replaces `{}` placeholders sequentially with the provided displayable arguments.
pub fn t_fmt(key: &str, args: &[&dyn std::fmt::Display]) -> String {
    let fmt_str = t(key);
    let mut result = fmt_str;
    for arg in args {
        result = result.replacen("{}", &arg.to_string(), 1);
    }
    result
}


/// Loads and parses a TOML translation file from the embedded assets.
fn load_translations(lang_code: &str) -> Option<Translations> {
    let filename = format!("{}.toml", lang_code);
    Asset::get(&filename).and_then(|file| {
        let contents = std::str::from_utf8(file.data.as_ref()).ok()?;
        toml::from_str(contents).ok()
    })
} 