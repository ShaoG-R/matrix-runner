use once_cell::sync::Lazy;
use std::sync::Mutex;

include!(concat!(env!("OUT_DIR"), "/i18n.rs"));

static CURRENT_LANG: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new("en".to_string()));

pub fn init(lang_code: &str) {
    let mut lang = CURRENT_LANG.lock().unwrap();
    // A simple check to see if the lang code might be valid.
    // The build script ensures `en` always exists.
    if lang_code == "en" || lang_code == "zh-CN" {
        *lang = lang_code.to_string();
    } else {
        *lang = "en".to_string();
    }
}

pub fn t(key: I18nKey) -> String {
    let lang_code = CURRENT_LANG.lock().unwrap();
    get_translation(&lang_code, key).to_string()
}

pub fn t_fmt(key: I18nKey, args: &[&dyn std::fmt::Display]) -> String {
    let base_translation = t(key);
    let mut result = base_translation;
    for (i, arg) in args.iter().enumerate() {
        let placeholder = format!("{{{}}}", i);
        result = result.replace(&placeholder, &arg.to_string());
    }
    result
} 