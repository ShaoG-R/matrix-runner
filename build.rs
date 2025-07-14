use serde::Deserialize;
use std::collections::BTreeMap;
use std::env;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct Translations(BTreeMap<String, String>);

fn to_pascal_case(s: &str) -> String {
    let mut pascal = String::with_capacity(s.len());
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

fn main() -> std::io::Result<()> {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let dest_path = Path::new(&out_dir).join("i18n.rs");

    let locales_dir = Path::new("locales");

    // 1. 一次性读取所有 .toml 文件路径
    let lang_files: Vec<PathBuf> = fs::read_dir(locales_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("toml"))
        .collect();

    // 2. 读取并解析基础语言文件 (en.toml)
    let base_lang_path = locales_dir.join("en.toml");
    let base_content = fs::read_to_string(&base_lang_path)?;
    let base_translations: Translations =
        toml::from_str(&base_content).expect("Failed to parse en.toml");

    let mut final_code = String::new();

    // 3. 优化：预先计算所有 key 的 PascalCase 形式
    //    同时，直接从 BTreeMap 的 key 生成 Enum，避免创建 HashSet
    let pascal_case_keys: BTreeMap<_, _> = base_translations
        .0
        .keys()
        .map(|key| (key.clone(), to_pascal_case(key)))
        .collect();

    // 生成 Enum
    writeln!(
        &mut final_code,
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\npub enum I18nKey {{"
    )
    .unwrap();
    for pascal_name in pascal_case_keys.values() {
        writeln!(&mut final_code, "    {},", pascal_name).unwrap();
    }
    writeln!(&mut final_code, "}}\n").unwrap();

    // 4. 优化：合并循环，一次性处理所有语言文件
    let mut lang_codes_for_dispatch = Vec::new();

    for path in &lang_files {
        let lang_code_raw = path.file_stem().unwrap().to_str().unwrap();
        let fn_lang_code = lang_code_raw.replace('-', "_").to_lowercase();

        lang_codes_for_dispatch.push((lang_code_raw.to_string(), fn_lang_code.clone()));

        let content = fs::read_to_string(path)?;
        let translations: Translations = toml::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse {:?}: {}", path, e));

        // 生成该语言的翻译函数
        writeln!(
            &mut final_code,
            "pub fn get_translation_{}(key: I18nKey) -> &'static str {{",
            fn_lang_code
        )
        .unwrap();
        writeln!(&mut final_code, "    match key {{").unwrap();

        // 使用预先计算好的 PascalCase key
        for (key, value) in &translations.0 {
            if let Some(pascal_key) = pascal_case_keys.get(key) {
                writeln!(
                    &mut final_code,
                    "        I18nKey::{} => r#\"{}\"#,",
                    pascal_key, value
                )
                .unwrap();
            }
            // 可选：如果 en.toml 中不存在某个 key，可以在这里发出警告
            // else {
            //     println!("cargo:warning=Key '{}' in {:?} not found in base (en.toml).", key, path);
            // }
        }
        writeln!(&mut final_code, "    }}\n}}\n").unwrap();
    }

    // 5. 使用收集到的信息生成主分发函数
    writeln!(
        &mut final_code,
        "pub fn get_translation(lang: &str, key: I18nKey) -> &'static str {{"
    )
    .unwrap();
    writeln!(&mut final_code, "    match lang {{").unwrap();
    for (raw_code, fn_code) in lang_codes_for_dispatch {
        writeln!(
            &mut final_code,
            "        r#\"{}\"# => get_translation_{}(key),",
            raw_code, fn_code
        )
        .unwrap();
    }
    // 确保 'en' 作为默认回退选项存在
    let en_fn_code = "en".to_string();
    writeln!(
        &mut final_code,
        "        _ => get_translation_{}(key),",
        en_fn_code
    )
    .unwrap();
    writeln!(&mut final_code, "    }}\n}}").unwrap();

    fs::write(&dest_path, final_code)?;
    println!("cargo:rerun-if-changed=locales/");

    Ok(())
}
