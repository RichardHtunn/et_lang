mod lexer;
mod generator;

use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo run <filename.et>");
        return;
    }

    let filename = &args[1];
    println!("--- Compiling {} ---", filename);

    let et_code = fs::read_to_string(filename)
        .expect("Error: Could not read the file.");

    let tokens = lexer::tokenize(&et_code);
    let cpp_output = generator::generate_cpp(tokens);

    let output_filename = filename.replace(".et", ".ino");
    fs::write(&output_filename, cpp_output)
        .expect("Error: Could not write the output file.");

    println!("Success! Saved to {}", output_filename);
}