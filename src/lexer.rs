#[derive(Debug, Clone)]
pub struct Line {
    pub number: usize,
    pub indent: usize,
    pub text: String,
}

pub fn tokenize(source: &str) -> Result<Vec<Line>, String> {
    let mut lines = Vec::new();
    for (index, raw) in source.lines().enumerate() {
        let number = index + 1;
        let indent = raw
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .map(|c| if c == '\t' { 4 } else { 1 })
            .sum();
        let text = raw.trim().split('#').next().unwrap_or("").trim();
        if text.is_empty() {
            continue;
        }
        lines.push(Line {
            number,
            indent,
            text: text.to_string(),
        });
    }
    Ok(lines)
}
