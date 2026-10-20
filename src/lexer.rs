#[derive(Debug, Clone)]
pub struct Line {
    pub number: usize,
    pub indent: usize,
    pub text: String,
    pub raw: String,
}

pub fn tokenize(source: &str) -> Result<Vec<Line>, String> {
    let mut lines = Vec::new();
    let mut raw_block_indent = None;
    for (index, raw) in source.lines().enumerate() {
        let number = index + 1;
        let indent = raw
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .map(|c| if c == '\t' { 4 } else { 1 })
            .sum();
        let trimmed = raw.trim();
        if let Some(parent_indent) = raw_block_indent {
            if trimmed.is_empty() || indent > parent_indent {
                lines.push(Line {
                    number,
                    indent,
                    text: trimmed.to_string(),
                    raw: raw.to_string(),
                });
                continue;
            }
            raw_block_indent = None;
        }
        let text = strip_comment(trimmed).trim();
        if text.is_empty() {
            continue;
        }
        if text == "cpp:" || text == "arduino:" {
            raw_block_indent = Some(indent);
        }
        lines.push(Line {
            number,
            indent,
            text: text.to_string(),
            raw: raw.to_string(),
        });
    }
    Ok(lines)
}

fn strip_comment(text: &str) -> &str {
    let mut quote = None;
    let mut escaped = false;
    for (index, ch) in text.char_indices() {
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == delimiter {
                quote = None;
            }
        } else if ch == '"' || ch == '\'' {
            quote = Some(ch);
        } else if ch == '#' {
            return &text[..index];
        }
    }
    text
}
