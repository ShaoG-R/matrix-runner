use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fmt::Write;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct Translations(BTreeMap<String, String>);

fn to_pascal_case(s: &str) -> String {
    let mut pascal = String::new();
    let mut capitalize = true;
    for c in s.chars() {
        if c == '_' {
            capitalize = true;
        } else if capitalize {
            pascal.push(c.to_ascii_uppercase());
            capitalize = false;
        } else {
            pascal.push(c);
        }
    }
    pascal
}

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("i18n.rs");

    let locales_dir = Path::new("locales");
    let mut lang_files = vec![];
    for entry in fs::read_dir(locales_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("toml") {
            lang_files.push(path);
        }
    }

    let base_lang_path = locales_dir.join("en.toml");
    let base_content = fs::read_to_string(&base_lang_path).unwrap();
    let base_translations: Translations = toml::from_str(&base_content).unwrap();
    let base_keys: HashSet<_> = base_translations.0.keys().cloned().collect();

    let mut final_code = String::new();

    // Generate Enum
    write!(
        &mut final_code,
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\npub enum I18nKey {{\n"
    )
    .unwrap();
    for key in &base_keys {
        writeln!(&mut final_code, "    {},", to_pascal_case(key)).unwrap();
    }
    write!(&mut final_code, "}}\n\n").unwrap();

    // Generate translation functions
    for path in &lang_files {
        let lang_code = path.file_stem().unwrap().to_str().unwrap().replace('-', "_").to_lowercase();
        let content = fs::read_to_string(path).unwrap();
        let translations: Translations = toml::from_str(&content).unwrap();
        
        writeln!(
            &mut final_code,
            "pub fn get_translation_{lang_code}(key: I18nKey) -> &'static str {{"
        )
        .unwrap();
        writeln!(&mut final_code, "    match key {{").unwrap();

        for (key, value) in &translations.0 {
            writeln!(
                &mut final_code,
                "        I18nKey::{} => r#\"{}\"#,",
                to_pascal_case(key),
                value
            )
            .unwrap();
        }
        write!(&mut final_code, "    }}\n}}\n\n").unwrap();
    }
    
    // Generate main dispatch function
    writeln!(&mut final_code, "pub fn get_translation(lang: &str, key: I18nKey) -> &'static str {{").unwrap();
    writeln!(&mut final_code, "    match lang {{").unwrap();
    for path in &lang_files {
        let lang_code = path.file_stem().unwrap().to_str().unwrap();
        let fn_lang_code = lang_code.replace('-', "_").to_lowercase();
        writeln!(&mut final_code, "        r#\"{lang_code}\"# => get_translation_{fn_lang_code}(key),").unwrap();
    }
    writeln!(&mut final_code, "        _ => get_translation_en(key),").unwrap(); // Fallback to English
    write!(&mut final_code, "    }}\n}}\n").unwrap();


    fs::write(&dest_path, final_code).unwrap();
    println!("cargo:rerun-if-changed=locales/");
}   