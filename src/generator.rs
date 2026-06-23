use crate::parser::{Program, Stmt};

pub fn generate_cpp(program: Program) -> String {
    let mut cpp = String::new();
    let mut loop_counter = 0; // Needed to prevent variable collision in repeat blocks
    
    cpp.push_str("#include <Arduino.h>\n\n");

    for stmt in &program.main_loop {
        if let Stmt::Set(name, val) = stmt {
            cpp.push_str(&format!("int {} = {};\n", name, val));
        }
    }
    cpp.push_str("\n");

    for task in &program.tasks {
        let args: Vec<String> = task.params.iter().map(|p| format!("int {}", p)).collect();
        cpp.push_str(&format!("void {}({}) {{\n", task.name, args.join(", ")));
        for stmt in &task.body {
            cpp.push_str(&generate_stmt(stmt, 1, &mut loop_counter));
        }
        cpp.push_str("}\n\n");
    }

    cpp.push_str("void setup() {\n");
    for stmt in &program.main_loop {
        if let Stmt::PinMode(pin, mode) = stmt {
            let cpp_mode = if mode == "out" { "OUTPUT" } else { "INPUT" };
            cpp.push_str(&format!("  pinMode({}, {});\n", pin, cpp_mode));
        }
    }
    cpp.push_str("}\n\n");

    cpp.push_str("void loop() {\n");
    for stmt in &program.main_loop {
        match stmt {
            Stmt::Set(_, _) | Stmt::PinMode(_, _) => continue,
            _ => cpp.push_str(&generate_stmt(stmt, 1, &mut loop_counter)),
        }
    }
    cpp.push_str("}\n");

    cpp
}

// A recursive function that generates perfectly indented C++
fn generate_stmt(stmt: &Stmt, indent: usize, loop_counter: &mut usize) -> String {
    let prefix = "  ".repeat(indent);
    let mut code = String::new();

    match stmt {
        Stmt::TurnOn(pin) => code.push_str(&format!("{}digitalWrite({}, HIGH);\n", prefix, pin)),
        Stmt::TurnOff(pin) => code.push_str(&format!("{}digitalWrite({}, LOW);\n", prefix, pin)),
        Stmt::Wait(time) => code.push_str(&format!("{}delay({});\n", prefix, time)),
        Stmt::TaskCall(name, args) => code.push_str(&format!("{}{}({});\n", prefix, name, args.join(", "))),
        Stmt::Power(pin, val) => code.push_str(&format!("{}analogWrite({}, {});\n", prefix, pin, val)),
        Stmt::Read(pin, var) => code.push_str(&format!("{}int {} = digitalRead({});\n", prefix, var, pin)),
        Stmt::MathOp(var, expr) => code.push_str(&format!("{}{} = {};\n", prefix, var, expr)),
        
        Stmt::RepeatBlock(times, body) => {
            let var_name = format!("i{}", loop_counter);
            *loop_counter += 1;
            code.push_str(&format!("{}for (int {} = 0; {} < {}; {}++) {{\n", prefix, var_name, var_name, times, var_name));
            for s in body { code.push_str(&generate_stmt(s, indent + 1, loop_counter)); }
            code.push_str(&format!("{}}}\n", prefix));
        }
        
        Stmt::IfBlock(cond, if_body, else_body) => {
            code.push_str(&format!("{}if ({}) {{\n", prefix, cond));
            for s in if_body { code.push_str(&generate_stmt(s, indent + 1, loop_counter)); }
            code.push_str(&format!("{}}}\n", prefix));
            
            if !else_body.is_empty() {
                code.push_str(&format!("{}else {{\n", prefix));
                for s in else_body { code.push_str(&generate_stmt(s, indent + 1, loop_counter)); }
                code.push_str(&format!("{}}}\n", prefix));
            }
        }
        _ => {}
    }
    code
}