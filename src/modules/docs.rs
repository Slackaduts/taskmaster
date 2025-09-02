use serde_yml::Value;

/// Adds markdown header data to a given identifier string, with depth determining the header level.
///
/// (Larger depth = smaller text)
fn add_header(identifier: &str, depth: i8) -> String {
    // Check if the identifier contains "data" and return an empty string if it does
    if identifier.contains("data") {
        return String::new();
    }

    let norm_i = identifier.replace('+', "");

    let mut header = String::new();
    for _ in 0..depth {
        header.push('#');
    }
    header.push(' ');
    header.push_str(&norm_i);
    header.push('\n');

    header
}


/// Converts all non-nested variants of a Value to its string representation, and format them in Markdown.
/// 
/// # Panic
/// Will panic!() if given any form of nested data structure.
/// (This was not designed with those in mind, use format_element())
/// 
/// Will handle meta tags from Taskmasker task data and format them accordingly.
fn format_value(data: &Value, identifier: &str) -> String {
    let display_str = match data {
        Value::Bool(b) => format!("{}", b),
        Value::Mapping(_) => panic!("format_value was called with a Mapping. This should not happen."),
        Value::Sequence(_) => panic!("format_value was called with a Sequence. This should not happen."),
        Value::String(s) => s.to_string(),
        Value::Tagged(_) => panic!("format_value was called with a Tagged. This should not happen."),
        Value::Number(n) => format!("{}", n),
        Value::Null => "null".to_string()
    };

    return match identifier.to_lowercase().as_ref() {
        "+description" => format!("{}\n", display_str),
        "+aliases" => format!("- {}\n", display_str), // Aliases will be bullets
        "+data" => String::new(), // This is for other TM operations
        "+example" => format!("```yaml\n{}\n```\n", display_str), // Code blocks, hopefully Obsidian has syntax highlighting
        "+passthru" | "+passthrough" => String::new(), // This is for other TM operations
        &_ => format!("{}\n", display_str), // Blindly pass through all data as it must be preserved
    }
}

/// Trims carriage return and newline from a given string slice.
fn trim_md_str(input: &str) -> String {
    let trim_pattern = "\r\n";
    return input.trim_end_matches(&trim_pattern).to_owned();
}

/// Converts a serde data structure into Markdown.
/// 
/// Taskmaster meta tags handle features like lists and code blocks.
/// 
/// Depth is handled by the depth of the recursion on the data structure.
pub fn format_element(data: &Value, identifier: &str, depth: i8) -> String {
    let mut md = String::new();
    
    if data.is_mapping() {
        md += &add_header(identifier, depth);
        let mapping = match data.as_mapping() {
            Some(m) => m,
            None => return trim_md_str(&md)
        };

        for key in mapping.keys() {
            let key_str = match key.as_str() {
                Some(k) => k,
                None => return trim_md_str(&md)
            };

            md += &format_element(&mapping[key], key_str, depth + 1);
        }

        return trim_md_str(&md)
    };

    if data.is_sequence() {
        md += &add_header(identifier, depth);
        let seq = match data.as_sequence() {
            Some(s) => s,
            None => return trim_md_str(&md)
        };

        for elem in seq.iter() {
            md += &format_element(elem, identifier, depth + 1);
        }

        return trim_md_str(&md)
    }

    let l_i = identifier.to_lowercase();
    if l_i.contains("+description") || l_i.contains("+example") {
        md += &add_header(identifier, depth);
    }

    md += &format_value(data, identifier);
    trim_md_str(&md)
}