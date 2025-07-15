//! # Build Script for Internationalization / 国际化构建脚本
//!
//! This build script generates Rust code for internationalization (i18n) support
//! by parsing TOML translation files and creating compile-time constants and enums.
//! It processes all `.toml` files in the `locales/` directory and generates
//! type-safe translation keys and lookup functions.
//!
//! 此构建脚本通过解析 TOML 翻译文件并创建编译时常量和枚举来生成国际化（i18n）支持的 Rust 代码。
//! 它处理 `locales/` 目录中的所有 `.toml` 文件，并生成类型安全的翻译键和查找函数。
//!
//! ## Generated Code / 生成的代码
//!
//! - `I18nKey` enum with all translation keys
//! - `get_translation()` function for key lookup
//! - Compile-time validation of translation completeness
//!
//! - 包含所有翻译键的 `I18nKey` 枚举
//! - 用于键查找的 `get_translation()` 函数
//! - 翻译完整性的编译时验证

use serde::Deserialize;
use std::collections::BTreeMap;
use std::env;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

/// Represents the structure of a translation file.
/// Maps translation keys to their localized strings.
///
/// 表示翻译文件的结构。
/// 将翻译键映射到其本地化字符串。
#[derive(Debug, Deserialize)]
struct Translations(BTreeMap<String, String>);

/// Converts a snake_case string to PascalCase.
/// Used to transform translation keys into enum variant names.
///
/// 将 snake_case 字符串转换为 PascalCase。
/// 用于将翻译键转换为枚举变体名称。
///
/// # Arguments / 参数
/// * `s` - The snake_case string to convert / 要转换的 snake_case 字符串
///
/// # Returns / 返回值
/// A PascalCase string / 一个 PascalCase 字符串
///
/// # Examples / 示例
/// ```
/// assert_eq!(to_pascal_case("hello_world"), "HelloWorld");
/// assert_eq!(to_pascal_case("test_case"), "TestCase");
/// ```
fn to_pascal_case(s: &str) -> String {
    let mut pascal = String::with_capacity(s.len());
    let mut capitalize = true;

    for c in s.chars() {
        if c == '_' {
            // Underscore indicates the next character should be capitalized
            // 下划线表示下一个字符应该大写
            capitalize = true;
        } else if capitalize {
            // Capitalize this character
            // 将此字符大写
            pascal.push(c.to_ascii_uppercase());
            capitalize = false;
        } else {
            // Keep the character as is
            // 保持字符原样
            pascal.push(c);
        }
    }

    pascal
}

/// Main build script function that generates internationalization code.
/// Processes all translation files and creates type-safe Rust code for i18n support.
///
/// 生成国际化代码的主构建脚本函数。
/// 处理所有翻译文件并为 i18n 支持创建类型安全的 Rust 代码。
///
/// # Process / 处理过程
/// 1. Read all .toml files from the locales directory
/// 2. Parse the base language file (en.toml) to extract all keys
/// 3. Generate an enum with all translation keys
/// 4. Generate lookup functions for each supported language
/// 5. Write the generated code to the output directory
///
/// 1. 从 locales 目录读取所有 .toml 文件
/// 2. 解析基础语言文件（en.toml）以提取所有键
/// 3. 生成包含所有翻译键的枚举
/// 4. 为每种支持的语言生成查找函数
/// 5. 将生成的代码写入输出目录
fn main() -> std::io::Result<()> {
    // Get the output directory for generated files
    // 获取生成文件的输出目录
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let dest_path = Path::new(&out_dir).join("i18n.rs");

    // Define the locales directory path
    // 定义 locales 目录路径
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
