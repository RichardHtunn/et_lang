#[derive(Debug, Clone)]
pub enum Token {
    SetVar { name: String, value: String },
    SetupBlock,
    LoopBlock,
    PinMode { pin: String, mode: String },
    TurnOn { pin: String },
    TurnOff { pin: String },
    Wait { amount: String },
    ReadPin { pin: String, var: String },
    IfBlock { condition: String },
    ElseBlock,
    RepeatBlock { times: String },
    Power { pin: String, value: String },
    MathOp { expression: String },     
    CloseBlock,
}

pub fn tokenize(code: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut indent_stack: Vec<usize> = Vec::new();

    for line in code.lines() {
        let stripped = line.trim();
        if stripped.is_empty() || stripped.starts_with('#') { continue; }

        let line_indent = line.len() - line.trim_start().len();

        if stripped == "setup:" || stripped == "loop:" {
            while !indent_stack.is_empty() {
                indent_stack.pop();
                tokens.push(Token::CloseBlock);
            }
        } else {
            while !indent_stack.is_empty() && line_indent < *indent_stack.last().unwrap() {
                indent_stack.pop();
                tokens.push(Token::CloseBlock);
            }
        }

        let parts: Vec<&str> = stripped.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "setup:" => tokens.push(Token::SetupBlock),
            "loop:" => tokens.push(Token::LoopBlock),
            "set" => {
                if parts.len() == 3 {
                    tokens.push(Token::SetVar { name: parts[1].to_string(), value: parts[2].to_string() });
                }
            }
            "pin" => {
                if parts.len() == 3 {
                    tokens.push(Token::PinMode { pin: parts[1].to_string(), mode: parts[2].to_string() });
                }
            }
            "on" => {
                if parts.len() == 2 {
                    tokens.push(Token::TurnOn { pin: parts[1].to_string() });
                }
            }
            "off" => {
                if parts.len() == 2 {
                    tokens.push(Token::TurnOff { pin: parts[1].to_string() });
                }
            }
            "wait" => {
                if parts.len() == 2 {
                    tokens.push(Token::Wait { amount: parts[1].to_string() });
                }
            }
            "read" => {
                if parts.len() == 4 && parts[2] == "into" {
                    tokens.push(Token::ReadPin { pin: parts[1].to_string(), var: parts[3].to_string() });
                }
            }
            "if" => {
                let mut condition = parts[1..].join(" ");
                if condition.ends_with(':') { condition.pop(); }
                tokens.push(Token::IfBlock { condition });
                indent_stack.push(line_indent + 1);
            }
            "else:" | "else" => {
                tokens.push(Token::ElseBlock);
                indent_stack.push(line_indent + 1);
            }
            "repeat" => { 
                let mut times = parts[1].to_string();
                if times.ends_with(':') { times.pop(); }
                tokens.push(Token::RepeatBlock { times });
                indent_stack.push(line_indent + 1);
            }
            "power" => {
                if parts.len() == 3 {
                    tokens.push(Token::Power { pin: parts[1].to_string(), value: parts[2].to_string() });
                }
            }
            "math" => {
                let expression = parts[1..].join(" ");
                tokens.push(Token::MathOp { expression });
            }
            _ => println!("Warning: Unknown command '{}'", stripped),
        }
    }

    while !indent_stack.is_empty() {
        indent_stack.pop();
        tokens.push(Token::CloseBlock);
    }

    tokens
}