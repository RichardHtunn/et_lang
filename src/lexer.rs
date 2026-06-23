#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // V1.0 Tokens
    Setup,
    Loop,
    Set(String, String),
    Pin(String, String),
    On(String),
    Off(String),
    Wait(String),
    Power(String, String),
    Read(String, String),
    Math(String, String),
    If(String),
    Else,
    Repeat(String),
    Identifier(String),
    Number(String),
    Indent(usize),
    
    // 🚀 NEW V2.0 Tokens
    Task,         // The 'task' keyword
    LParen,       // '('
    RParen,       // ')'
    Comma,        // ','
}

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    
    for line in input.lines() {
        // Skip comments and empty lines
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }

        // 1. Calculate Indentation Level (Python-style block structure)
        let indent_level = line.len() - line.trim_start().len();
        tokens.push(Token::Indent(indent_level));

        // 2. Process words and special symbols
        // Splitting tokens while preserving parentheses and commas
        let processed_line = trimmed
            .replace("(", " ( ")
            .replace(")", " ) ")
            .replace(",", " , ");
            
        let words: Vec<&str> = processed_line.split_whitespace().collect();
        let mut i = 0;
        
        while i < words.len() {
            match words[i] {
                // Core Structural Elements
                "setup:" => tokens.push(Token::Setup),
                "loop:" => tokens.push(Token::Loop),
                
                // V2.0 Structural Keywords & Symbols
                "task" => tokens.push(Token::Task),
                "(" => tokens.push(Token::LParen),
                ")" => tokens.push(Token::RParen),
                "," => tokens.push(Token::Comma),
                
                // Existing V1.0 Commands
                "set" => {
                    tokens.push(Token::Set(words[i+1].to_string(), words[i+2].to_string()));
                    i += 2;
                }
                "pin" => {
                    tokens.push(Token::Pin(words[i+1].to_string(), words[i+2].to_string()));
                    i += 2;
                }
                "on" => {
                    tokens.push(Token::On(words[i+1].to_string()));
                    i += 1;
                }
                "off" => {
                    tokens.push(Token::Off(words[i+1].to_string()));
                    i += 1;
                }
                "wait" => {
                    tokens.push(Token::Wait(words[i+1].to_string()));
                    i += 1;
                }
                "power" => {
                    tokens.push(Token::Power(words[i+1].to_string(), words[i+2].to_string()));
                    i += 2;
                }
                
                // Fallback for names, variables, and values
                other => {
                    // Check if it's a number or just a raw identifier variable name
                    if other.chars().all(|c| c.is_numeric()) {
                        tokens.push(Token::Number(other.to_string()));
                    } else {
                        // Strip trailing colon if it's part of an 'if' syntax
                        let clean_name = other.trim_end_matches(':');
                        tokens.push(Token::Identifier(clean_name.to_string()));
                    }
                }
            }
            i += 1;
        }
    }
    tokens
}