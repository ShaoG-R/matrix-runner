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

/// 检测系统默认语言
/// Detects the system's default language using the sys-locale crate
///
/// # Returns / 返回值
/// * `String` - 检测到的语言代码，如果检测失败则返回 "en"
///              The detected language code, or "en" if detection fails
///
/// # 检测逻辑 / Detection Logic
/// 1. 使用 sys-locale crate 获取系统语言设置
/// 2. 解析语言代码并映射到支持的语言
/// 3. 如果检测失败或不支持，回退到 "en"
///
/// 1. Use sys-locale crate to get system language settings
/// 2. Parse language code and map to supported languages
/// 3. If detection fails or unsupported, fallback to "en"
pub fn detect_system_language() -> String {
    // 使用 sys-locale 获取系统语言
    // Use sys-locale to get system language
    match sys_locale::get_locale() {
        Some(locale) => {
            let lang_code = parse_language_from_locale(&locale);
            if is_supported_language(&lang_code) {
                lang_code
            } else {
                "en".to_string()
            }
        }
        None => "en".to_string(),
    }
}

/// 从 BCP 47 language tag 中解析语言代码
/// Parses language code from BCP 47 language tag
///
/// BCP 47 格式：language[-script][-region][-variant]
/// BCP 47 format: language[-script][-region][-variant]
///
/// # Examples / 示例
/// * `zh-CN` -> `zh-CN` (简体中文，中国)
/// * `zh-TW` -> `zh-CN` (繁体中文，台湾 -> 映射到简体中文)
/// * `zh-Hans` -> `zh-CN` (简体中文脚本)
/// * `zh-Hant` -> `zh-CN` (繁体中文脚本 -> 映射到简体中文)
/// * `zh-Hans-CN` -> `zh-CN` (简体中文脚本，中国)
/// * `zh-Hant-TW` -> `zh-CN` (繁体中文脚本，台湾 -> 映射到简体中文)
/// * `en` -> `en` (英文)
/// * `en-US` -> `en` (英文，美国)
/// * `en-GB` -> `en` (英文，英国)
/// * `fr-FR` -> `en` (法文 -> 回退到英文)
fn parse_language_from_locale(locale: &str) -> String {
    // 将 locale 转换为小写以便比较，并按 '-' 分割
    // Convert locale to lowercase for comparison and split by '-'
    let locale_lower = locale.to_lowercase();
    let parts: Vec<&str> = locale_lower.split('-').collect();

    if parts.is_empty() {
        return "en".to_string();
    }

    let language = parts[0];

    match language {
        "zh" => {
            // 中文处理：检查脚本和地区代码
            // Chinese handling: check script and region codes
            for part in &parts[1..] {
                match *part {
                    // 繁体中文脚本或台湾地区 -> 暂时映射到简体中文
                    // Traditional Chinese script or Taiwan region -> temporarily map to Simplified Chinese
                    "hant" | "tw" => return "zh-CN".to_string(),
                    // 简体中文脚本或中国大陆地区
                    // Simplified Chinese script or Mainland China region
                    "hans" | "cn" => return "zh-CN".to_string(),
                    _ => continue,
                }
            }
            // 如果没有特定的脚本或地区信息，默认为简体中文
            // If no specific script or region info, default to Simplified Chinese
            "zh-CN".to_string()
        }
        "en" => {
            // 英文：不区分地区，统一使用 "en"
            // English: no region distinction, use "en" uniformly
            "en".to_string()
        }
        _ => {
            // 其他语言暂时映射到英文
            // Other languages temporarily mapped to English
            "en".to_string()
        }
    }
}

/// 检查语言代码是否被支持
/// Checks if a language code is supported
fn is_supported_language(lang_code: &str) -> bool {
    matches!(lang_code, "en" | "zh-CN")
}



/// 一个支持 {n} 和 {} 两种占位符的格式化函数。
///
/// ## 参数
/// - `key`: 用于查找翻译模板的键。
/// - `args`: 一个包含所有要填充的参数的切片。
///
/// ## 替换规则
/// 1. 对于每个参数 `args[i]`，函数会优先查找并替换模板中的 `{i}`。
/// 2. 如果模板中不存在 `{i}`，该参数 `args[i]` 则会被用于替换模板中下一个可用的 `{}`。
pub fn t_fmt(key: I18nKey, args: &[&dyn fmt::Display]) -> String {
    let result = t(key);
    fmt_core(&*result, args)
}

use std::fmt::{self, Write};

/// 定义了解析后的片段类型
enum Segment<'a> {
    /// 纯文本切片
    Literal(&'a str),
    /// 带编号的占位符，值为其在参数列表中的索引
    Indexed(usize),
    /// 不带编号的占位符
    Unindexed,
}

/// 格式化一个字符串，支持带编号的占位符（如 {0}）和不带编号的占位符（{}）。
/// 这是一个高性能版本，只对模板字符串进行一次遍历来解析，然后高效构建结果。
///
/// ## 参数
/// - `s`: 包含占位符的模板字符串。
/// - `args`: 用于替换占位符的参数切片。
///
/// ## 替换规则 (与原始版本逻辑一致)
/// 1. 对于每个参数 `args[i]`，函数会优先查找并替换模板中的 `{i}`。
/// 2. 如果模板中不存在 `{i}`，该参数 `args[i]` 则会被用于替换模板中下一个可用的 `{}`。
/// 3. `{{` 和 `}}` 被视作转义的大括号，不会被替换，最终会原样输出。
fn fmt_core(s: &str, args: &[&dyn fmt::Display]) -> String {
    // 预估最终字符串的容量，以减少内存重分配的次数。
    // 初始容量为模板长度，并为每个参数增加一个保守的估计值（例如16字节）。
    let mut result = String::with_capacity(s.len() + args.len() * 16);

    // 存储解析出的片段
    let mut segments = Vec::new();
    // 标记已被带编号占位符使用的参数
    let mut used_by_index = vec![false; args.len()];

    // --- 解析阶段：单次遍历模板字符串 ---
    let mut last_end = 0;
    while last_end < s.len() {
        // 从当前位置查找下一个需要处理的字符：'{' 或 '}'
        let search_area = &s[last_end..];
        let next_brace = search_area.find(|c| c == '{' || c == '}');

        let Some(brace_offset) = next_brace else {
            // 后面没有大括号了，剩余部分全部是字面量
            break;
        };

        let brace_index = last_end + brace_offset;

        // 推入在大括号之前的字面量
        if last_end < brace_index {
            segments.push(Segment::Literal(&s[last_end..brace_index]));
        }

        // 检查是否是 '{{' 或 '}}'
        if s[brace_index..].starts_with("{{") {
            // `{{` 被视为字面量 `{{`，并跳过占位符解析
            segments.push(Segment::Literal("{{"));
            last_end = brace_index + 2;
        } else if s[brace_index..].starts_with("}}") {
            // `}}` 被视为字面量 `}}`
            segments.push(Segment::Literal("}}"));
            last_end = brace_index + 2;
        } else if s[brace_index..].starts_with('{') {
            // 是一个占位符的开始
            let placeholder_area = &s[brace_index..];
            if let Some(end_offset) = placeholder_area[1..].find('}') {
                let end_index = brace_index + 1 + end_offset;
                let content = &s[brace_index + 1..end_index];

                if content.is_empty() {
                    segments.push(Segment::Unindexed);
                } else if let Ok(idx) = content.parse::<usize>() {
                    if idx < args.len() {
                        segments.push(Segment::Indexed(idx));
                        used_by_index[idx] = true;
                    } else {
                        // 索引越界，视为字面量
                        segments.push(Segment::Literal(&s[brace_index..=end_index]));
                    }
                } else {
                    // 内容不是数字，视为字面量
                    segments.push(Segment::Literal(&s[brace_index..=end_index]));
                }
                last_end = end_index + 1;
            } else {
                // 未闭合的 '{'，视为字面量
                segments.push(Segment::Literal("{"));
                last_end = brace_index + 1;
            }
        } else {
            // 单独的 '}'，视为字面量
            segments.push(Segment::Literal("}"));
            last_end = brace_index + 1;
        }
    }

    // 添加最后一个文本片段
    if last_end < s.len() {
        segments.push(Segment::Literal(&s[last_end..]));
    }

    // --- 构建阶段 ---
    // 创建一个迭代器，只提供未被带编号占位符使用的参数
    let mut unindexed_args = args.iter().enumerate()
        .filter(|(i, _)| !used_by_index[*i])
        .map(|(_, arg)| arg);

    for segment in segments {
        match segment {
            Segment::Literal(text) => result.push_str(text),
            Segment::Indexed(index) => {
                // 使用 write! 宏来处理格式化，这比 .to_string() 更高效
                let _ = write!(result, "{}", args[index]);
            }
            Segment::Unindexed => {
                if let Some(arg) = unindexed_args.next() {
                    let _ = write!(result, "{}", arg);
                } else {
                    // 如果没有更多可用参数，则将 '{}' 作为字面量输出
                    result.push_str("{}");
                }
            }
        }
    }

    result
}


// 仅在运行 `cargo test` 时编译此模块
#[cfg(test)]
mod tests {
    use super::*; // 导入父模块中的所有内容，包括 fmt_core

    #[test]
    fn test_parse_language_from_locale() {
        // 测试中文 BCP 47 language tags
        // Test Chinese BCP 47 language tags
        assert_eq!(parse_language_from_locale("zh"), "zh-CN");
        assert_eq!(parse_language_from_locale("zh-CN"), "zh-CN");
        assert_eq!(parse_language_from_locale("zh-TW"), "zh-CN"); // 繁体映射到简体
        assert_eq!(parse_language_from_locale("zh-Hans"), "zh-CN");
        assert_eq!(parse_language_from_locale("zh-Hant"), "zh-CN"); // 繁体映射到简体
        assert_eq!(parse_language_from_locale("zh-Hans-CN"), "zh-CN");
        assert_eq!(parse_language_from_locale("zh-Hant-TW"), "zh-CN"); // 繁体映射到简体
        assert_eq!(parse_language_from_locale("zh-Hans-SG"), "zh-CN"); // 新加坡简体中文

        // 测试英文 BCP 47 language tags
        // Test English BCP 47 language tags
        assert_eq!(parse_language_from_locale("en"), "en");
        assert_eq!(parse_language_from_locale("en-US"), "en");
        assert_eq!(parse_language_from_locale("en-GB"), "en");
        assert_eq!(parse_language_from_locale("en-AU"), "en");
        assert_eq!(parse_language_from_locale("en-CA"), "en");

        // 测试大小写不敏感
        // Test case insensitivity
        assert_eq!(parse_language_from_locale("ZH-CN"), "zh-CN");
        assert_eq!(parse_language_from_locale("EN-US"), "en");
        assert_eq!(parse_language_from_locale("Zh-Hans-CN"), "zh-CN");

        // 测试其他语言（应该回退到英文）
        // Test other languages (should fallback to English)
        assert_eq!(parse_language_from_locale("fr"), "en");
        assert_eq!(parse_language_from_locale("fr-FR"), "en");
        assert_eq!(parse_language_from_locale("de-DE"), "en");
        assert_eq!(parse_language_from_locale("ja-JP"), "en");
        assert_eq!(parse_language_from_locale("ko-KR"), "en");
        assert_eq!(parse_language_from_locale("es-ES"), "en");

        // 测试边界情况
        // Test edge cases
        assert_eq!(parse_language_from_locale(""), "en");
        assert_eq!(parse_language_from_locale("-"), "en");
        assert_eq!(parse_language_from_locale("invalid"), "en");
    }

    #[test]
    fn test_is_supported_language() {
        assert!(is_supported_language("en"));
        assert!(is_supported_language("zh-CN"));
        assert!(!is_supported_language("fr"));
        assert!(!is_supported_language("de"));
        assert!(!is_supported_language("ja"));
    }

    #[test]
    fn test_detect_system_language() {
        // 这个测试只是确保函数不会 panic，并返回有效的语言代码
        let detected = detect_system_language();
        assert!(is_supported_language(&detected));
    }

    // --- 基本功能测试 ---

    #[test]
    fn test_simple_unnumbered() {
        let s = "Hello, {}!";
        let name = "World";
        assert_eq!(fmt_core(s, &[&name]), "Hello, World!");
    }

    #[test]
    fn test_multiple_unnumbered() {
        let s = "User: {}, Role: {}";
        let name = "Alice";
        let role = "Admin";
        assert_eq!(fmt_core(s, &[&name, &role]), "User: Alice, Role: Admin");
    }

    #[test]
    fn test_simple_numbered_in_order() {
        let s = "User: {0}, Role: {1}";
        let name = "Bob";
        let role = "Moderator";
        assert_eq!(fmt_core(s, &[&name, &role]), "User: Bob, Role: Moderator");
    }

    #[test]
    fn test_simple_numbered_out_of_order() {
        let s = "Role: {1}, User: {0}";
        let name = "Charlie";
        let role = "Guest";
        assert_eq!(fmt_core(s, &[&name, &role]), "Role: Guest, User: Charlie");
    }

    #[test]
    fn test_reuse_numbered_placeholder() {
        let s = "Name: {0}, please confirm your name is {0}.";
        let name = "David";
        assert_eq!(fmt_core(s, &[&name]), "Name: David, please confirm your name is David.");
    }

    #[test]
    fn test_mixed_placeholders() {
        // 规则：优先处理带编号的，然后按顺序处理不带编号的
        let s = "Runner {1} processes {} cases for target {0}.";
        let target = "x86_64";
        let runner_id = 3;
        let case_count = 150;
        // args[0] = target, args[1] = runner_id, args[2] = case_count
        let expected = "Runner 3 processes 150 cases for target x86_64.";
        assert_eq!(fmt_core(s, &[&target, &runner_id, &case_count]), expected);
    }

    #[test]
    fn test_different_arg_types() {
        let s = "Name: {}, Age: {}, Height: {}";
        let name = "Eve";
        let age = 30;
        let height = 1.75;
        let expected = "Name: Eve, Age: 30, Height: 1.75";
        assert_eq!(fmt_core(s, &[&name, &age, &height]), expected);
    }

    // --- 边缘情况测试 ---

    #[test]
    fn test_no_placeholders_no_args() {
        let s = "A plain string.";
        assert_eq!(fmt_core(s, &[]), "A plain string.");
    }

    #[test]
    fn test_no_placeholders_with_args() {
        let s = "A plain string.";
        let arg = "some value";
        assert_eq!(fmt_core(s, &[&arg]), "A plain string.");
    }

    #[test]
    fn test_with_placeholders_no_args() {
        let s = "Hello, {0} and {}!";
        assert_eq!(fmt_core(s, &[]), "Hello, {0} and {}!");
    }

    #[test]
    fn test_more_args_than_placeholders() {
        let s = "Value: {}";
        let arg1 = "one";
        let arg2 = "two"; // 这个参数应该被忽略
        assert_eq!(fmt_core(s, &[&arg1, &arg2]), "Value: one");
    }

    #[test]
    fn test_fewer_args_than_placeholders() {
        let s = "Values: {}, {}, {}";
        let arg1 = "one";
        let arg2 = "two";
        let expected = "Values: one, two, {}"; // 最后一个占位符应保留
        assert_eq!(fmt_core(s, &[&arg1, &arg2]), expected);
    }

    #[test]
    fn test_fewer_args_than_numbered_placeholders() {
        let s = "Values: {0}, {1}, {2}";
        let arg1 = "one";
        let arg2 = "two";
        let expected = "Values: one, two, {2}"; // {2} 应保留
        assert_eq!(fmt_core(s, &[&arg1, &arg2]), expected);
    }

    #[test]
    fn test_empty_string_input() {
        let s = "";
        let arg = "some value";
        assert_eq!(fmt_core(s, &[&arg]), "");
    }

    #[test]
    fn test_string_with_braces_not_placeholders() {
        let s = "This is a {JSON} object, not a placeholder.";
        let arg = "test";
        // 因为没有 {0} 或 {}，所以不应发生替换
        assert_eq!(fmt_core(s, &[&arg]), s);
    }

    #[test]
    fn test_escaped_looking_placeholders() {
        // `replace` 和 `replacen` 不会处理转义，所以 {{0}} 不会匹配 {0}
        let s = "This is {{0}} and {{}}";
        let arg = "test";
        assert_eq!(fmt_core(s, &[&arg]), s);
    }

    #[test]
    fn test_argument_contains_placeholder() {
        let s = "First: {}, Second: {}";
        let arg1 = "{1}";
        let arg2 = "value";
        let expected = "First: {1}, Second: value";
        assert_eq!(fmt_core(s, &[&arg1, &arg2]), expected);
    }

    #[test]
    fn test_unicode_in_template_and_args() {
        let s = "你好, {}. 你有 {1} 条新消息：'{}'。";
        let name = "世界"; // args[0] -> {}
        let message = "你好，Rust！"; // args[1] -> {1}
        let emoji = "🚀"; // args[2] -> {}
        let expected = "你好, 世界. 你有 你好，Rust！ 条新消息：'🚀'。";
        assert_eq!(fmt_core(s, &[&name, &message, &emoji]), expected);
    }
}
