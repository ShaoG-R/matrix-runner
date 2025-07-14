use once_cell::sync::OnceCell;
use rust_embed::RustEmbed;
use std::collections::HashMap;

/// Embeds the translation files from the `locales/` directory into the binary.
/// This allows the application to be distributed as a single executable file
/// without needing separate language files.
/// 将 `locales/` 目录下的翻译文件嵌入到二进制文件中。
/// 这使得应用程序可以作为单个可执行文件分发，而无需附带独立的语言文件。
#[derive(RustEmbed)]
#[folder = "locales/"]
struct Asset;

/// A type alias for a map holding key-value translation pairs.
/// The key is the translation key (e.g., "welcome_message"), and the value
/// is the translated string.
/// 用于保存键值对翻译的映射的类型别名。
/// 键是翻译键（例如 "welcome_message"），值是翻译后的字符串。
type Translations = HashMap<String, String>;

/// A thread-safe, lazily-initialized static variable to hold the loaded translations.
/// `OnceCell` ensures that the translations are loaded only once.
/// 一个线程安全的、延迟初始化的静态变量，用于存放加载的翻译。
/// `OnceCell` 确保翻译只被加载一次。
static TRANSLATIONS: OnceCell<Translations> = OnceCell::new();

/// Initializes the translation system with a specified language.
/// It loads the corresponding `.toml` file from the embedded assets. If the
/// specified language is not found or is empty, it defaults to "en".
///
/// # Panics
/// Panics if the default "en" translation file cannot be loaded, as it's
/// considered a critical part of the application.
///
/// 使用指定的语言初始化翻译系统。
/// 它会从嵌入的资源中加载对应的 `.toml` 文件。如果指定的语言未找到或为空，
/// 则默认使用 "en"。
///
/// # Panics
/// 如果默认的 "en" 翻译文件无法加载，则会 panic，因为它被认为是应用程序的关键部分。
pub fn init(language: &str) {
    let lang_code = if language.is_empty() { "en" } else { language };

    let translations = load_translations(lang_code).unwrap_or_else(|| {
        if lang_code != "en" {
            // Fallback to English if the specified language file is not found.
            // 如果指定的语言文件未找到，则回退到英语。
            load_translations("en").expect("Failed to load default English translations.")
        } else {
            // This should ideally not happen if en.toml is guaranteed to be embedded.
            // 如果 en.toml 保证被嵌入，这理论上不应该发生。
            panic!("Failed to load default English translations.");
        }
    });

    TRANSLATIONS.set(translations).expect("Failed to set translations");
}

/// Retrieves a translated string for a given key.
///
/// # Panics
/// - Panics if the translation system has not been initialized with `init()` first.
/// - Panics if the key is not found in the loaded translations, to ensure all
///   UI text is properly defined.
///
/// 根据给定的键获取翻译后的字符串。
///
/// # Panics
/// - 如果翻译系统未首先通过 `init()` 初始化，则会 panic。
/// - 如果在加载的翻译中找不到该键，则会 panic，以确保所有 UI 文本都已正确定义。
pub fn t(key: &str) -> String {
    let translations = TRANSLATIONS.get().expect("I18n system not initialized. Call init() first.");
    translations.get(key).cloned().unwrap_or_else(|| panic!("Translation key not found: {}", key))
}

/// Retrieves and formats a translated string with dynamic arguments.
/// It replaces `{}` placeholders in the translated string sequentially with the
/// provided displayable arguments.
///
/// # Arguments
/// * `key` - The key for the format string in the translation files.
/// * `args` - A slice of arguments that implement the `Display` trait.
///
/// # Example
/// `t_fmt("welcome_user", &["Alice"])` might produce "Welcome, Alice!".
///
/// 获取并格式化带有动态参数的翻译字符串。
/// 它会按顺序将翻译字符串中的 `{}` 占位符替换为提供的可显示参数。
///
/// # Arguments
/// * `key` - 翻译文件中格式字符串的键。
/// * `args` - 一个实现了 `Display` trait 的参数切片。
///
/// # Example
/// `t_fmt("welcome_user", &["Alice"])` 可能会生成 "Welcome, Alice!"。
pub fn t_fmt(key: &str, args: &[&dyn std::fmt::Display]) -> String {
    let fmt_str = t(key);
    let mut result = fmt_str;
    for arg in args {
        // Sequentially replace the next occurrence of `{}`.
        // 按顺序替换下一个出现的 `{}`。
        result = result.replacen("{}", &arg.to_string(), 1);
    }
    result
}

/// Loads and parses a TOML translation file from the embedded assets.
/// It takes a language code, finds the corresponding file, reads it, and
/// deserializes it from TOML format into a `Translations` HashMap.
///
/// # Returns
/// Returns `Some(Translations)` on success, or `None` if the file is not found
/// or fails to parse.
///
/// 从嵌入的资源中加载并解析一个 TOML 翻译文件。
/// 它接受一个语言代码，找到对应的文件，读取它，并将其从 TOML 格式反序列化
/// 为一个 `Translations` HashMap。
///
/// # Returns
/// 成功时返回 `Some(Translations)`，如果文件未找到或解析失败，则返回 `None`。
fn load_translations(lang_code: &str) -> Option<Translations> {
    let filename = format!("{}.toml", lang_code);
    Asset::get(&filename).and_then(|file| {
        let contents = std::str::from_utf8(file.data.as_ref()).ok()?;
        toml::from_str(contents).ok()
    })
} 