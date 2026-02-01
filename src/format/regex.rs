/// Escape curly braces inside LaTeX math expressions for MDX compatibility
pub fn escape_curly_braces_in_math(content: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = content.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '$' {
            // Check if it's display math ($$)
            let is_display = i + 1 < chars.len() && chars[i + 1] == '$';
            let delimiter_len = if is_display { 2 } else { 1 };

            // Find closing delimiter
            let mut j = i + delimiter_len;
            let mut found_close = false;

            while j < chars.len() {
                if chars[j] == '$' {
                    if is_display && j + 1 < chars.len() && chars[j + 1] == '$' {
                        found_close = true;
                        break;
                    } else if !is_display {
                        found_close = true;
                        break;
                    }
                }
                j += 1;
            }

            if found_close {
                // Add opening delimiter
                for _ in 0..delimiter_len {
                    result.push('$');
                }

                // Escape braces in math content
                for k in (i + delimiter_len)..j {
                    if chars[k] == '{' || chars[k] == '}' {
                        // Check if already escaped
                        if k == 0 || chars[k - 1] != '\\' {
                            result.push('\\');
                        }
                    }
                    result.push(chars[k]);
                }

                // Add closing delimiter
                for _ in 0..delimiter_len {
                    result.push('$');
                }

                i = j + delimiter_len;
                continue;
            }
        }

        result.push(chars[i]);
        i += 1;
    }

    result
}

// /// Convert Hugo details shortcode to Fumadocs Accordion components
// fn convert_hugo_details_to_accordion(content: &str) -> String {
//     let mut result = content.to_string();

//     // First, handle single-line shortcodes: {{% details title="..." %}} content {{% /details %}}
//     let re_single_line =
//         Regex::new(r#"\{\{% details title="([^"]*)"[^%]*%\}\}\s*(.+?)\s*\{\{% /details %\}\}"#)
//             .unwrap();
//     result = re_single_line
//         .replace_all(&result, "<Accordion title=\"$1\">\n$2\n</Accordion>")
//         .to_string();

//     // Convert opening tags
//     let re_open = Regex::new(r#"\{\{% details title="([^"]*)"[^%]*%\}\}"#).unwrap();
//     result = re_open
//         .replace_all(&result, r#"<Accordion title="$1">"#)
//         .to_string();

//     // Convert closing tags - ensure they're on their own line for MDX compatibility
//     // Replace any occurrence where {{% /details %}} appears at end of line content
//     let re_closing = Regex::new(r#"([^\n])\s*\{\{% /details %\}\}"#).unwrap();
//     result = re_closing
//         .replace_all(&result, "$1\n</Accordion>")
//         .to_string();

//     // Handle any remaining standalone closing tags
//     result = result.replace("{{% /details %}}", "</Accordion>");

//     // Wrap consecutive Accordion blocks in Accordions
//     result = wrap_accordions_in_container(&result);

//     result
// }

/// Wrap consecutive Accordion blocks in a single Accordions container
pub fn wrap_accordions_in_container(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut in_sequence = false;
    let mut accordion_buffer = Vec::new();
    let mut depth = 0;

    for (i, line) in lines.iter().enumerate() {
        if line.contains("<Accordion ") && !in_sequence {
            // Start of accordion sequence
            in_sequence = true;
            accordion_buffer.push(line.to_string());
            depth = 1;
        } else if in_sequence {
            accordion_buffer.push(line.to_string());

            // Track depth
            if line.contains("<Accordion ") {
                depth += 1;
            }
            if line.contains("</Accordion>") {
                depth -= 1;
            }

            // Check if sequence ends
            if depth == 0 {
                // Look ahead to see if next non-empty line is another Accordion
                let mut next_is_accordion = false;
                for j in (i + 1)..lines.len() {
                    let next_line = lines[j].trim();
                    if next_line.is_empty() {
                        continue;
                    }
                    if next_line.contains("<Accordion ") {
                        next_is_accordion = true;
                    }
                    break;
                }

                if !next_is_accordion {
                    // End of sequence - wrap and flush
                    result.push("<Accordions>".to_string());
                    result.extend(accordion_buffer.drain(..));
                    result.push("</Accordions>".to_string());
                    in_sequence = false;
                }
            }
        } else {
            result.push(line.to_string());
        }
    }

    // Handle case where file ends with accordion sequence
    if !accordion_buffer.is_empty() {
        result.push("<Accordions>".to_string());
        result.extend(accordion_buffer);
        result.push("</Accordions>".to_string());
    }

    result.join("\n")
}
