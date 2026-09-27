mod avr;
mod lexer;
mod parser;

use std::{
    env, fs,
    path::PathBuf,
    process::{Command, ExitCode},
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("et: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args.len() > 2 {
        return Err("usage: et <file.et> [serial-port]".into());
    }
    let path = PathBuf::from(&args[0]);
    if path.extension().and_then(|s| s.to_str()) != Some("et") {
        return Err("source file must have a .et extension".into());
    }
    let source = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let lines = lexer::tokenize(&source)?;
    let program = parser::Parser::new(lines).parse_program()?;
    let bytes = avr::compile(&program)?;
    let output = path.with_extension("hex");
    fs::write(&output, avr::intel_hex(&bytes)).map_err(|e| format!("{}: {e}", output.display()))?;
    println!(
        "Compiled {} to {} ({} flash bytes)",
        path.display(),
        output.display(),
        bytes.len()
    );

    if let Some(port) = args.get(1) {
        let status = Command::new("avrdude")
            .args([
                "-p", "m328p", "-c", "arduino", "-P", port, "-b", "115200", "-D", "-U",
            ])
            .arg(format!("flash:w:{}:i", output.display()))
            .status()
            .map_err(|e| format!("could not start avrdude: {e}"))?;
        if !status.success() {
            return Err(format!("avrdude failed with status {status}"));
        }
        println!("Uploaded to {port}");
    }
    Ok(())
}
