use crate::lexer::Token;

pub fn generate_cpp(tokens: Vec<Token>) -> String {
    let mut global_code = Vec::new();
    let mut setup_code = Vec::new();
    let mut loop_code = Vec::new();
    
    let mut current_block = "global";
    let mut loop_counter = 0;
    let mut current_indent = 1;

    for token in tokens {
        match token {
            Token::SetupBlock => { current_block = "setup"; current_indent = 1; }
            Token::LoopBlock => { current_block = "loop"; current_indent = 1; }
            Token::SetVar { name, value } => {
                global_code.push(format!("int {} = {};", name, value));
            }
            Token::PinMode { pin, mode } => {
                let cpp_mode = if mode == "out" { "OUTPUT" } else { "INPUT" };
                let prefix = "  ".repeat(current_indent);
                let line = format!("{}pinMode({}, {});", prefix, pin, cpp_mode);
                if current_block == "setup" { setup_code.push(line); } else { loop_code.push(line); }
            }
            Token::TurnOn { pin } => {
                let prefix = "  ".repeat(current_indent);
                let line = format!("{}digitalWrite({}, HIGH);", prefix, pin);
                if current_block == "setup" { setup_code.push(line); } else { loop_code.push(line); }
            }
            Token::TurnOff { pin } => {
                let prefix = "  ".repeat(current_indent);
                let line = format!("{}digitalWrite({}, LOW);", prefix, pin);
                if current_block == "setup" { setup_code.push(line); } else { loop_code.push(line); }
            }
            Token::Wait { amount } => {
                let prefix = "  ".repeat(current_indent);
                let line = format!("{}delay({});", prefix, amount);
                if current_block == "setup" { setup_code.push(line); } else { loop_code.push(line); }
            }
            Token::ReadPin { pin, var } => {
                let prefix = "  ".repeat(current_indent);
                let line = format!("{}int {} = digitalRead({});", prefix, var, pin);
                if current_block == "setup" { setup_code.push(line); } else { loop_code.push(line); }
            }
            Token::IfBlock { condition } => {
                let prefix = "  ".repeat(current_indent);
                let line = format!("{}if ({}) {{", prefix, condition);
                if current_block == "setup" { setup_code.push(line); } else { loop_code.push(line); }
                current_indent += 1;
            }
            Token::ElseBlock => {
                let prefix = "  ".repeat(current_indent);
                let line = format!("{}else {{", prefix);
                if current_block == "setup" { setup_code.push(line); } else { loop_code.push(line); }
                current_indent += 1;
            }
            Token::RepeatBlock { times } => {
                let var_name = format!("i{}", loop_counter);
                loop_counter += 1;
                let prefix = "  ".repeat(current_indent);
                let line = format!("{}for (int {} = 0; {} < {}; {}++) {{", prefix, var_name, var_name, times, var_name);
                if current_block == "setup" { setup_code.push(line); } else { loop_code.push(line); }
                current_indent += 1;
            }
            Token::Power { pin, value } => {
                let prefix = "  ".repeat(current_indent);
                let line = format!("{}analogWrite({}, {});", prefix, pin, value);
                if current_block == "setup" { setup_code.push(line); } else { loop_code.push(line); }
            }
            Token::MathOp { expression } => {
                let prefix = "  ".repeat(current_indent);
                let line = format!("{}{};", prefix, expression);
                if current_block == "setup" { setup_code.push(line); } else { loop_code.push(line); }
            }
            Token::CloseBlock => {
                if current_indent > 1 { current_indent -= 1; }
                let prefix = "  ".repeat(current_indent);
                let line = format!("{}}}", prefix);
                if current_block == "setup" { setup_code.push(line); } else { loop_code.push(line); }
            }
        }
    }

    let mut final_cpp = String::new();
    final_cpp.push_str("#include <Arduino.h>\n\n");
    
    for line in global_code { final_cpp.push_str(&format!("{}\n", line)); }
    if !final_cpp.ends_with("\n\n") { final_cpp.push_str("\n"); }

    final_cpp.push_str("void setup() {\n");
    for line in setup_code { final_cpp.push_str(&format!("{}\n", line)); }
    final_cpp.push_str("}\n\n");

    final_cpp.push_str("void loop() {\n");
    for line in loop_code { final_cpp.push_str(&format!("{}\n", line)); }
    final_cpp.push_str("}\n");

    final_cpp
}