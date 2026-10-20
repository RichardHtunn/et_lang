mod avr;
mod lexer;
mod ml;
mod nano;
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
    if let Some(board) = program.board.as_deref() {
        if args.len() == 2 {
            return Err(
                "Arduino sketch output cannot use the serial-port argument; upload it with Arduino tooling".into(),
            );
        }
        let sketch = if let Some(raw) = &program.arduino_sketch {
            raw.clone()
        } else {
            nano::emit(&program)?
        };
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| "source filename must have a valid stem".to_string())?;
        let output_dir = PathBuf::from("target").join("et").join(stem);
        fs::create_dir_all(&output_dir).map_err(|e| format!("{}: {e}", output_dir.display()))?;
        let output = output_dir.join(format!("{stem}.ino"));
        fs::write(&output, sketch).map_err(|e| format!("{}: {e}", output.display()))?;
        let fqbn = match board {
            "uno_r3" => "arduino:avr:uno",
            "uno_r4_minima" => "arduino:renesas_uno:minima",
            "nano33blesense_rev2" => "arduino:mbed_nano:nano33ble",
            _ => unreachable!("parser validates boards"),
        };
        println!(
            "Generated {} for {} ({fqbn})",
            output.display(),
            path.display()
        );
        return Ok(());
    }
    let source_dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let models = ml::load_models(&program.models, source_dir)?;
    for spec in &program.models {
        let model = models[&spec.name];
        let direction = if model.one_when_high { ">=" } else { "<" };
        println!(
            "Learned {}: label 1 when value {} {} ({} of {} training examples correct)",
            spec.name, direction, model.threshold, model.correct, model.total
        );
    }
    let bytes = avr::compile(&program, &models)?;
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
