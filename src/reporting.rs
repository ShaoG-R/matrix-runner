//! # Reporting Module / 报告模块
//!
//! This module handles the generation and display of test reports in multiple formats.
//! It provides functionality for creating styled HTML reports and printing colorful,
//! formatted summaries to the console with internationalization support.
//!
//! 此模块处理多种格式的测试报告生成和显示。
//! 它提供创建样式化 HTML 报告和在控制台打印彩色格式化摘要的功能，支持国际化。

pub mod console;
pub mod html;

// Re-export common reporting functions
pub use console::{print_summary, print_unexpected_failure_details};
pub use html::generate_html_report; 