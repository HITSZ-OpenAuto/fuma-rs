import re

with open('src/formatter.rs', 'r') as f:
    content = f.read()

def replace_func(func_name, code):
    # Find the function in the file
    pattern = rf"(fn {func_name}\(content: &str\) -> String {{\n)(.*?)(^\}})"
    match = re.search(pattern, content, re.DOTALL | re.MULTILINE)
    if not match:
        print(f"Could not find {func_name}")
        return content
    return content[:match.start()] + f"fn {func_name}(content: &str) -> String {{\n" + code + "}" + content[match.end():]

# fix_self_closing_tags
fix_self_closing_tags_code = """    let mut result: Cow<str> = Cow::Borrowed(content);

    // Convert <br> to <br />
    let re_br = Regex::new(r"<br\\s*>").unwrap();
    if let Cow::Owned(s) = re_br.replace_all(&result, "<br />") {
        result = Cow::Owned(s);
    }

    // Convert <hr> to <hr />
    let re_hr = Regex::new(r"<hr\\s*>").unwrap();
    if let Cow::Owned(s) = re_hr.replace_all(&result, "<hr />") {
        result = Cow::Owned(s);
    }

    result.into_owned()
"""
content = replace_func("fix_self_closing_tags", fix_self_closing_tags_code)

# fix_malformed_html
fix_malformed_html_code = """    let mut result: Cow<str> = Cow::Borrowed(content);

    // Remove empty <tr> tags before closing table
    let re_tr_table = Regex::new(r"<tr>\\s*</table>").unwrap();
    if let Cow::Owned(s) = re_tr_table.replace_all(&result, "</table>") {
        result = Cow::Owned(s);
    }

    // Remove empty <tr></tr> tags
    let re_empty_tr = Regex::new(r"<tr>\\s*</tr>").unwrap();
    if let Cow::Owned(s) = re_empty_tr.replace_all(&result, "") {
        result = Cow::Owned(s);
    }

    result.into_owned()
"""
content = replace_func("fix_malformed_html", fix_malformed_html_code)

# convert_hugo_callout_shortcodes
convert_hugo_callout_shortcodes_code = """    let mut result: Cow<str> = Cow::Borrowed(content);

    // Remove opening callout tags such as:
    // {{< callout type="info" >}} or {{% callout type="warning" %}}
    let re_open = Regex::new(r"\\{\\{[<%]\\s*callout\\b[^{}]*[>%]\\}\\}").unwrap();
    if let Cow::Owned(s) = re_open.replace_all(&result, "") {
        result = Cow::Owned(s);
    }

    // Remove closing callout tags such as:
    // {{< /callout >}} or {{% /callout %}}
    let re_close = Regex::new(r"\\{\\{[<%]\\s*/callout\\s*[>%]\\}\\}").unwrap();
    if let Cow::Owned(s) = re_close.replace_all(&result, "") {
        result = Cow::Owned(s);
    }

    result.into_owned()
"""
content = replace_func("convert_hugo_callout_shortcodes", convert_hugo_callout_shortcodes_code)

# convert_hugo_details_to_accordion
convert_hugo_details_to_accordion_code = """    let mut result: Cow<str> = Cow::Borrowed(content);

    // First, handle single-line shortcodes: {{% details title="..." %}} content {{% /details %}}
    let re_single_line =
        Regex::new(r#"\\{\\{% details title="([^"]*)"[^%]*%\\}\\}\\s*(.+?)\\s*\\{\\{% /details %\\}\\}"#)
            .unwrap();
    if let Cow::Owned(s) = re_single_line.replace_all(&result, "<Accordion title=\\"$1\\">\\n$2\\n</Accordion>") {
        result = Cow::Owned(s);
    }

    // Convert opening tags
    let re_open = Regex::new(r#"\\{\\{% details title="([^"]*)"[^%]*%\\}\\}"#).unwrap();
    if let Cow::Owned(s) = re_open.replace_all(&result, r#"<Accordion title="$1">"#) {
        result = Cow::Owned(s);
    }

    // Convert closing tags - ensure they're on their own line for MDX compatibility
    // Replace any occurrence where {{% /details %}} appears at end of line content
    let re_closing = Regex::new(r#"([^\\n])\\s*\\{\\{% /details %\\}\\}"#).unwrap();
    if let Cow::Owned(s) = re_closing.replace_all(&result, "$1\\n</Accordion>") {
        result = Cow::Owned(s);
    }

    let mut result_owned = result.into_owned();
    // Handle any remaining standalone closing tags
    result_owned = result_owned.replace("{{% /details %}}", "</Accordion>");

    // Wrap consecutive Accordion blocks in Accordions
    result_owned = wrap_accordions_in_container(&result_owned);

    result_owned
"""
content = replace_func("convert_hugo_details_to_accordion", convert_hugo_details_to_accordion_code)


with open('src/formatter.rs', 'w') as f:
    f.write(content)
