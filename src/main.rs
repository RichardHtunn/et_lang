mod lexer;
mod parser; // NEW: Bring in the parser!
mod generator;

use std::env;
use std::fs;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo run <filename.et> [port]");
        return;
    }

    let filename = &args[1];
    println!("--- Compiling {} ---", filename);

    let et_code = fs::read_to_string(filename)
        .expect("Error: Could not read the file.");

    // 1. LEXER: Turn text into Tokens
    let tokens = lexer::tokenize(&et_code);
    
    // 2. PARSER: Turn Tokens into an Abstract Syntax Tree (AST)
    let mut parser = parser::Parser::new(tokens);
    let ast = parser.parse_program();
    
    // 3. GENERATOR: Turn the AST into C++
    let cpp_output = generator::generate_cpp(ast);

    // Auto-Folder Fix & Save File
    let base_name = filename.replace(".et", "");
    let folder_path = format!("./{}", base_name);
    let output_filename = format!("{}/{}.ino", folder_path, base_name);

    fs::create_dir_all(&folder_path).expect("Failed to create folder");
    fs::write(&output_filename, cpp_output).expect("Error writing file");
    
    println!("Success! V2.0 C++ code saved to {}", output_filename);

    // Auto-Flash Logic (Kept from your V1.2 update)
    if args.len() >= 3 {
        let port = &args[2];
        println!("--- Flashing directly to {} ---", port);

        let status = Command::new("arduino-cli")
            .arg("compile")
            .arg("--upload")
            .arg("-b")
            .arg("arduino:avr:uno") 
            .arg("-p")
            .arg(port)
            .arg(&folder_path)
            .status()
            .expect("Failed to run arduino-cli.");

        if status.success() {
            println!("⚡ Hardware successfully updated! ⚡");
        }
    }
}