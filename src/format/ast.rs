//! AST-based Markdown/MDX formatter
//!
//! This module provides a production-ready AST-based approach to formatting Markdown/MDX files.
//! It uses pulldown-cmark to parse and manipulate the document structure.
//!
//! ## How it works
//!
//! Instead of using regex to match patterns in raw text, this formatter:
//! 1. Parses Markdown into an event stream (similar to SAX parsing for XML)
//! 2. Processes each event with a state machine
//! 3. Filters/transforms events based on context
//! 4. Converts events back to Markdown
//!
//! ## Advantages over regex
//!
//! - **Context-aware**: Knows when you're inside code blocks, images, etc.
//! - **Structure-aware**: Understands nested elements correctly
//! - **More reliable**: Won't accidentally match patterns in wrong contexts

use pulldown_cmark::{CowStr, Event, Options, Parser, Tag, TagEnd};
use regex::Regex;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Convert Hugo shortcodes using regex (unavoidable - non-standard syntax)
///
/// Hugo shortcodes like `{{% details %}}` are not part of standard Markdown,
/// so no Markdown parser recognizes them. We must use regex to convert them
/// to MDX components before AST processing.
///
/// Transformations:
/// - `{{% details title="X" %}}` → `<Accordion title="X">`
/// - `{{% /details %}}` → `</Accordion>`
/// - Ensures closing tags are on their own line (MDX requirement)
fn convert_hugo_shortcodes(content: &str) -> String {
    let mut result = content.to_string();

    // Handle single-line shortcodes
    let re_single = Regex::new(
        r#"\{\{% details title="([^"]*)"[^%]*%\}\}\s*(.+?)\s*\{\{% /details %\}\}"#,
    )
    .unwrap();
    result = re_single
        .replace_all(&result, "<Accordion title=\"$1\">\n$2\n</Accordion>")
        .to_string();

    // Convert opening tags
    let re_open = Regex::new(r#"\{\{% details title="([^"]*)"[^%]*%\}\}"#).unwrap();
    result = re_open
        .replace_all(&result, r#"<Accordion title="$1">"#)
        .to_string();

    // Convert closing tags - ensure newline before
    let re_closing = Regex::new(r#"([^\n])\s*\{\{% /details %\}\}"#).unwrap();
    result = re_closing
        .replace_all(&result, "$1\n</Accordion>")
        .to_string();

    result.replace("{{% /details %}}", "</Accordion>")
}

/// Process Markdown with AST operations
///
/// This is the core of the AST-based formatter. It:
/// 1. Parses Markdown into an event stream
/// 2. Processes each event with state machine logic
/// 3. Filters out unwanted elements (comments, badges)
/// 4. Transforms HTML elements (fix tags, convert styles)
/// 5. Converts the modified event stream back to Markdown
fn process_with_ast(content: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_FOOTNOTES);

    let parser = Parser::new_ext(content, options);

    // Collect events with state machine processing
    let events = process_events(parser);

    // Convert events back to markdown
    events_to_markdown(events)
}

/// Process events with state machines for complex transformations
///
/// This function iterates through all parser events and uses a state machine
/// to track context (e.g., whether we're inside a code block or image).
/// Some events are filtered out (returning None), others are transformed.
fn process_events<'a>(parser: Parser<'a>) -> Vec<Event<'a>> {
    let mut events = Vec::new();
    let mut state = ProcessorState::new();

    for event in parser {
        match process_event(event, &mut state) {
            Some(processed) => events.push(processed),
            None => {} // Event filtered out
        }
    }

    events
}

/// State machine for event processing
///
/// This struct tracks the current parsing context to make context-aware decisions.
///
/// Fields:
/// - `in_code_block`: True when we're inside a code block (don't modify HTML there)
/// - `in_image`: True when we're inside an image element
/// - `current_image_url`: The URL of the current image being processed
/// - `skip_until_image_end`: True when filtering out a shields.io badge
struct ProcessorState {
    in_code_block: bool,
    in_image: bool,
    current_image_url: String,
    skip_until_image_end: bool,
}

impl ProcessorState {
    fn new() -> Self {
        Self {
            in_code_block: false,
            in_image: false,
            current_image_url: String::new(),
            skip_until_image_end: false,
        }
    }
}

/// Process a single event with state tracking
///
/// This is where the magic happens. For each event:
/// 1. Update state (track if we enter/exit code blocks, images, etc.)
/// 2. Decide whether to keep, filter, or transform the event
/// 3. Return Some(event) to keep it, None to filter it out
///
/// The state machine ensures we only modify content in the right context.
/// For example, we don't fix HTML tags inside code blocks.
fn process_event<'a>(event: Event<'a>, state: &mut ProcessorState) -> Option<Event<'a>> {
    // Track code blocks
    match &event {
        Event::Start(Tag::CodeBlock(_)) => state.in_code_block = true,
        Event::End(TagEnd::CodeBlock) => state.in_code_block = false,
        _ => {}
    }

    // Handle image state machine for badge removal
    match &event {
        Event::Start(Tag::Image {
            dest_url,
            title,
            link_type,
            ..
        }) => {
            state.in_image = true;
            state.current_image_url = dest_url.to_string();

            // Check if this is a shields.io badge
            if dest_url.contains("shields.io") {
                state.skip_until_image_end = true;
                return None; // Filter out the start tag
            }

            state.skip_until_image_end = false;
            return Some(Event::Start(Tag::Image {
                dest_url: dest_url.clone(),
                title: title.clone(),
                link_type: *link_type,
                id: CowStr::from(""),
            }));
        }
        Event::End(TagEnd::Image) => {
            state.in_image = false;
            if state.skip_until_image_end {
                state.skip_until_image_end = false;
                return None; // Filter out the end tag
            }
        }
        Event::Text(_) | Event::Code(_) if state.skip_until_image_end => {
            return None; // Skip image alt text for badges
        }
        _ => {}
    }

    // Process HTML events (both block and inline)
    match event {
        Event::Html(html) => {
            // Skip HTML comments
            if html.trim().starts_with("<!--") {
                return None;
            }

            // Don't modify HTML inside code blocks
            if state.in_code_block {
                return Some(Event::Html(html));
            }

            let mut fixed = html.to_string();

            // Fix self-closing tags
            fixed = fixed.replace("<br>", "<br />");
            fixed = fixed.replace("<hr>", "<hr />");

            // Fix malformed HTML
            fixed = fix_malformed_html_ast(&fixed);

            // Convert style attributes to JSX
            fixed = convert_style_to_jsx_ast(&fixed);

            Some(Event::Html(CowStr::from(fixed)))
        }
        Event::InlineHtml(html) => {
            // Skip HTML comments
            if html.trim().starts_with("<!--") {
                return None;
            }

            // Don't modify HTML inside code blocks
            if state.in_code_block {
                return Some(Event::InlineHtml(html));
            }

            let mut fixed = html.to_string();

            // Fix self-closing tags
            fixed = fixed.replace("<br>", "<br />");
            fixed = fixed.replace("<hr>", "<hr />");

            // Fix malformed HTML
            fixed = fix_malformed_html_ast(&fixed);

            // Convert style attributes to JSX
            fixed = convert_style_to_jsx_ast(&fixed);

            Some(Event::InlineHtml(CowStr::from(fixed)))
        }
        other => Some(other),
    }
}

/// Fix malformed HTML patterns
///
/// Fixes common HTML errors:
/// - `<tr></table>` → `</table>` (remove empty tr before table close)
/// - `<tr></tr>` → `` (remove empty tr tags)
fn fix_malformed_html_ast(html: &str) -> String {
    let mut result = html.to_string();

    // Remove empty <tr> before </table>
    let re_tr_table = Regex::new(r"<tr>\s*</table>").unwrap();
    result = re_tr_table.replace_all(&result, "</table>").to_string();

    // Remove empty <tr></tr>
    let re_empty_tr = Regex::new(r"<tr>\s*</tr>").unwrap();
    result = re_empty_tr.replace_all(&result, "").to_string();

    result
}

/// Convert HTML style attributes to JSX format
///
/// MDX requires style attributes to be JavaScript objects, not strings.
///
/// Transformation:
/// - `style="text-align:center; color:red"`
/// - → `style={{textAlign: "center", color: "red"}}`
///
/// Also converts CSS property names to camelCase (text-align → textAlign)
fn convert_style_to_jsx_ast(html: &str) -> String {
    let re = Regex::new(r#"style="([^"]*)""#).unwrap();

    re.replace_all(html, |caps: &regex::Captures| {
        let style_str = &caps[1];
        let mut jsx_props = Vec::new();

        for prop in style_str.split(';') {
            let prop = prop.trim();
            if prop.is_empty() || !prop.contains(':') {
                continue;
            }

            let parts: Vec<&str> = prop.splitn(2, ':').collect();
            if parts.len() == 2 {
                let name = css_to_camel_case(parts[0].trim());
                let value = parts[1].trim();
                jsx_props.push(format!("{}: \"{}\"", name, value));
            }
        }

        if jsx_props.is_empty() {
            String::new()
        } else {
            format!("style={{{{{}}}}}", jsx_props.join(", "))
        }
    })
    .to_string()
}

/// Convert CSS property name to camelCase
///
/// Examples:
/// - "text-align" → "textAlign"
/// - "background-color" → "backgroundColor"
/// - "margin" → "margin" (no change)
fn css_to_camel_case(prop: &str) -> String {
    let parts: Vec<&str> = prop.trim().split('-').collect();
    if parts.is_empty() {
        return String::new();
    }

    let mut result = parts[0].to_string();
    for part in &parts[1..] {
        if !part.is_empty() {
            let mut chars = part.chars();
            if let Some(first) = chars.next() {
                result.push(first.to_uppercase().next().unwrap());
                result.push_str(chars.as_str());
            }
        }
    }
    result
}

/// Format all MDX files in a directory recursively
///
/// This function:
/// 1. Walks through the directory tree
/// 2. Finds all .mdx files
/// 3. Formats each file with `format_mdx_complete`
/// 4. Only writes if content changed
/// 5. Returns count of modified files
///
/// Example:
/// ```
/// let docs_dir = Path::new("content/docs");
/// let count = format_all_mdx_files(&docs_dir)?;
/// println!("Formatted {} files", count);
/// ```
pub fn format_all_mdx_files(docs_dir: &Path) -> crate::error::Result<usize> {
    let mut modified_count = 0;

    for entry in WalkDir::new(docs_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "mdx"))
    {
        let path = entry.path();
        let original = fs::read_to_string(path)?;
        let formatted = format_mdx_complete(&original);

        if formatted != original {
            fs::write(path, formatted)?;
            modified_count += 1;
        }
    }

    Ok(modified_count)
}

/// Convert events back to Markdown string
///
/// Uses pulldown-cmark-to-cmark to serialize the event stream
/// back into Markdown text.
fn events_to_markdown(events: Vec<Event>) -> String {
    let mut buf = String::new();

    match pulldown_cmark_to_cmark::cmark(events.iter(), &mut buf) {
        Ok(_) => buf,
        Err(e) => {
            eprintln!("Warning: Failed to convert AST back to Markdown: {}", e);
            // Return empty string on error - caller should handle
            String::new()
        }
    }
}

/// Clean up multiple consecutive blank lines
///
/// Replaces 3+ consecutive newlines with exactly 2 newlines.
/// This keeps the document clean without removing intentional spacing.
fn cleanup_blank_lines(content: &str) -> String {
    let re = Regex::new(r"\n{3,}").unwrap();
    re.replace_all(content, "\n\n").to_string()
}

/// Escape curly braces in math expressions
///
/// MDX interprets `{` and `}` as JSX expressions, but LaTeX math uses them.
/// We need to escape them: `${x}$` → `$\{x\}$`
///
/// Note: pulldown-cmark doesn't support LaTeX math syntax natively,
/// so we use the manual character traversal from regex.rs module.
/// This is one case where a simple state machine (char-by-char) works
/// better than regex or AST.
pub fn escape_curly_braces_in_math(content: &str) -> String {
    super::regex::escape_curly_braces_in_math(content)
}

/// Wrap consecutive Accordion blocks in Accordions container
///
/// Fumadocs requires multiple `<Accordion>` elements to be wrapped in
/// a single `<Accordions>` container.
///
/// Example:
/// ```
/// <Accordion title="A">...</Accordion>
/// <Accordion title="B">...</Accordion>
/// ```
/// becomes:
/// ```
/// <Accordions>
/// <Accordion title="A">...</Accordion>
/// <Accordion title="B">...</Accordion>
/// </Accordions>
/// ```
///
/// This function tracks nesting depth to handle nested accordions correctly.
pub fn wrap_accordions_in_container(content: &str) -> String {
    super::regex::wrap_accordions_in_container(content)
}

/// Validate MDX syntax using AST parsing
///
/// Checks for common structural errors:
/// - Unclosed tags
/// - Unexpected closing tags
///
/// This can catch errors before they cause build failures.
#[allow(unused)]
pub fn validate_mdx(content: &str) -> Result<(), String> {
    let parser = Parser::new(content);
    let mut tag_stack: Vec<String> = Vec::new();

    for event in parser {
        match event {
            Event::Start(tag) => {
                tag_stack.push(format!("{:?}", tag));
            }
            Event::End(_) => {
                if tag_stack.is_empty() {
                    return Err("Unexpected closing tag found".to_string());
                }
                tag_stack.pop();
            }
            _ => {}
        }
    }

    if !tag_stack.is_empty() {
        return Err(format!("Unclosed tags: {:?}", tag_stack));
    }

    Ok(())
}

/// Complete formatting with AST + manual processing
///
/// This is the main entry point for formatting MDX files.
/// It combines AST processing with manual algorithms where needed.
///
/// Processing pipeline:
/// 1. **Hugo shortcodes** (regex) - Non-standard syntax, must use regex
/// 2. **AST processing** - Remove comments, fix HTML, remove badges
/// 3. **Math braces** (manual) - Character-by-character state machine
/// 4. **Wrap accordions** (manual) - Line-by-line with depth tracking
/// 5. **Cleanup** (regex) - Remove extra blank lines
///
/// This hybrid approach uses the best tool for each job:
/// - Regex for non-standard syntax and simple replacements
/// - AST for context-aware transformations
/// - Manual traversal for unsupported syntax (LaTeX math)
pub fn format_mdx_complete(content: &str) -> String {
    let mut result = content.to_string();

    // Phase 1: Hugo shortcodes (regex - non-standard syntax)
    result = convert_hugo_shortcodes(&result);

    // Phase 2: AST processing (standard Markdown)
    result = process_with_ast(&result);

    // Phase 3: Math braces (manual - not supported by parser)
    result = escape_curly_braces_in_math(&result);

    // Phase 4: Wrap accordions (manual - context-aware)
    result = wrap_accordions_in_container(&result);

    // Phase 5: Cleanup
    result = cleanup_blank_lines(&result);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_html_comments() {
        let input = "Text <!-- comment --> more";
        let output = process_with_ast(input);
        assert!(!output.contains("<!--"));
        assert!(output.contains("Text"));
    }

    #[test]
    fn test_fix_self_closing_tags() {
        let input = "Line <br> break <hr> rule";
        let output = process_with_ast(input);
        assert!(output.contains("<br />"));
        assert!(output.contains("<hr />"));
    }

    #[test]
    fn test_remove_shields_badges() {
        let input = "![Badge](https://img.shields.io/badge/test-blue) text";
        let output = process_with_ast(input);
        assert!(!output.contains("shields.io"));
    }

    #[test]
    fn test_keep_normal_images() {
        let input = "![Image](https://example.com/image.png)";
        let output = process_with_ast(input);
        assert!(output.contains("example.com"));
    }

    #[test]
    fn test_convert_style_to_jsx() {
        let input = r#"<div style="text-align:center;color:red;"></div>"#;
        let output = process_with_ast(input);
        assert!(output.contains("textAlign"));
        assert!(output.contains("{{"));
    }

    #[test]
    fn test_css_to_camel_case() {
        assert_eq!(css_to_camel_case("text-align"), "textAlign");
        assert_eq!(css_to_camel_case("background-color"), "backgroundColor");
        assert_eq!(css_to_camel_case("margin"), "margin");
    }

    #[test]
    fn test_state_machine_code_block() {
        let input = "Text <br> outside\n\n```html\n<br>\n```\n\nMore <br> text";
        let output = process_with_ast(input);

        // Should fix <br> outside code blocks
        let lines: Vec<&str> = output.lines().collect();
        let outside_lines: Vec<&str> = lines
            .iter()
            .filter(|l| !l.contains("```") && !l.trim().is_empty())
            .copied()
            .collect();

        // Check that HTML outside code blocks is fixed
        assert!(
            outside_lines.iter().any(|l| l.contains("<br />")),
            "Should fix <br> outside code blocks"
        );
    }

    #[test]
    fn test_validate_mdx_valid() {
        let valid = "# Title\n\nSome text\n\n- List item";
        assert!(validate_mdx(valid).is_ok());
    }

    #[test]
    fn test_cleanup_blank_lines() {
        let input = "line1\n\n\n\nline2\n\n\nline3";
        let output = cleanup_blank_lines(input);
        assert!(!output.contains("\n\n\n"));
    }

    #[test]
    fn test_convert_hugo_shortcodes() {
        let input = r#"{{% details title="Test" %}}Content{{% /details %}}"#;
        let output = convert_hugo_shortcodes(input);
        assert!(output.contains("<Accordion"));
        assert!(output.contains("</Accordion>"));
    }

    #[test]
    fn test_complete_format() {
        let input = r#"
# Title

<!-- comment -->

![Badge](https://img.shields.io/badge/test)

Text <br> here

{{% details title="Details" %}}
Content here
{{% /details %}}
"#;
        let output = format_mdx_complete(input);

        assert!(!output.contains("<!--"));
        assert!(!output.contains("shields.io"));
        assert!(output.contains("<br />"));
        assert!(output.contains("<Accordion"));
    }
}
