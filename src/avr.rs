use crate::parser::{Program, Stmt, StmtKind, Task};
use std::collections::HashMap;

const FLASH_WORDS: usize = 16_384;

#[derive(Clone, Copy)]
enum Binding {
    Const(u16),
    Var(u16),
}

pub fn compile(program: &Program) -> Result<Vec<u8>, String> {
    let mut initial = HashMap::new();
    let mut order = Vec::new();
    for (name, value, line) in &program.constants {
        if initial.contains_key(name) {
            return Err(format!("line {line}: duplicate name {name}"));
        }
        let number = resolve_initial(value, &initial, *line)?;
        initial.insert(name.clone(), number);
        order.push(name.clone());
    }

    let mut changed = std::collections::HashSet::new();
    let mut read_names = std::collections::BTreeSet::new();
    let mut uses_pwm = false;
    scan(&program.setup, &mut changed, &mut read_names, &mut uses_pwm);
    scan(
        &program.loop_body,
        &mut changed,
        &mut read_names,
        &mut uses_pwm,
    );
    for task in &program.tasks {
        scan(&task.body, &mut changed, &mut read_names, &mut uses_pwm);
    }
    for name in read_names {
        if !initial.contains_key(&name) {
            initial.insert(name.clone(), 0);
            order.push(name);
        }
    }

    let mut bindings = HashMap::new();
    let mut variables = Vec::new();
    for name in order {
        let value = initial[&name];
        if changed.contains(&name) {
            let address = 0x0100 + variables.len() as u16 * 2;
            if address > 0x08FE {
                return Err("too many variables for Uno SRAM".into());
            }
            bindings.insert(name, Binding::Var(address));
            variables.push((address, value));
        } else {
            bindings.insert(name, Binding::Const(value));
        }
    }

    let mut tasks = HashMap::new();
    for task in &program.tasks {
        if tasks.insert(task.name.as_str(), task).is_some() {
            return Err(format!("duplicate task {}", task.name));
        }
    }

    let mut compiler = Compiler {
        words: Vec::new(),
        tasks,
        call_stack: Vec::new(),
        uses_pwm,
    };
    for (address, value) in variables {
        compiler.ldi(24, value as u8);
        compiler.ldi(25, (value >> 8) as u8);
        compiler.store_word(address, 24);
    }
    if uses_pwm {
        // Timer1: phase-correct 8-bit PWM, prescaler 64. Channels start disconnected.
        compiler.ldi(24, 0x01);
        compiler.sts(0x0080, 24);
        compiler.ldi(24, 0x03);
        compiler.sts(0x0081, 24);
        compiler.ldi(24, 0);
        for address in [0x0089, 0x0088, 0x008B, 0x008A] {
            compiler.sts(address, 24);
        }
    }
    compiler.block(&program.setup, &bindings, 0)?;
    let loop_start = compiler.words.len();
    compiler.block(&program.loop_body, &bindings, 0)?;
    compiler.rjmp(loop_start, 0)?;
    if compiler.words.len() > FLASH_WORDS {
        return Err("program exceeds Arduino Uno's 32 KB flash".into());
    }
    let mut bytes = Vec::with_capacity(compiler.words.len() * 2);
    for word in compiler.words {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    Ok(bytes)
}

fn scan(
    statements: &[Stmt],
    changed: &mut std::collections::HashSet<String>,
    read_names: &mut std::collections::BTreeSet<String>,
    uses_pwm: &mut bool,
) {
    for stmt in statements {
        match &stmt.kind {
            StmtKind::Read(_, name) => {
                changed.insert(name.clone());
                read_names.insert(name.clone());
            }
            StmtKind::Math(name, _) => {
                changed.insert(name.clone());
            }
            StmtKind::Power(_, _) => *uses_pwm = true,
            StmtKind::Repeat(_, body) => scan(body, changed, read_names, uses_pwm),
            StmtKind::If(_, yes, no) => {
                scan(yes, changed, read_names, uses_pwm);
                scan(no, changed, read_names, uses_pwm);
            }
            _ => {}
        }
    }
}

fn resolve_initial(text: &str, values: &HashMap<String, u16>, line: usize) -> Result<u16, String> {
    if text.chars().all(|c| c.is_ascii_digit()) {
        return text
            .parse::<u16>()
            .map_err(|_| format!("line {line}: value must be 0 through 65535"));
    }
    values
        .get(text)
        .copied()
        .ok_or_else(|| format!("line {line}: unknown value {text}"))
}

fn value(text: &str, values: &HashMap<String, Binding>, line: usize) -> Result<Binding, String> {
    if text.chars().all(|c| c.is_ascii_digit()) {
        return text
            .parse::<u16>()
            .map(Binding::Const)
            .map_err(|_| format!("line {line}: value must be 0 through 65535"));
    }
    values
        .get(text)
        .copied()
        .ok_or_else(|| format!("line {line}: unknown value {text}"))
}

fn resolve(text: &str, values: &HashMap<String, Binding>, line: usize) -> Result<u16, String> {
    match value(text, values, line)? {
        Binding::Const(number) => Ok(number),
        Binding::Var(_) => Err(format!(
            "line {line}: {text} changes at runtime; this command needs a fixed value"
        )),
    }
}

struct Compiler<'a> {
    words: Vec<u16>,
    tasks: HashMap<&'a str, &'a Task>,
    call_stack: Vec<String>,
    uses_pwm: bool,
}
impl Compiler<'_> {
    fn emit(&mut self, word: u16) {
        self.words.push(word);
    }

    fn rjmp(&mut self, target: usize, line: usize) -> Result<(), String> {
        let here = self.words.len();
        self.emit(0);
        self.patch_rjmp(here, target, line)
    }

    fn patch_rjmp(&mut self, at: usize, target: usize, line: usize) -> Result<(), String> {
        let offset = target as isize - at as isize - 1;
        if !(-2048..=2047).contains(&offset) {
            return Err(format!("line {line}: jump is too far for this Uno backend"));
        }
        self.words[at] = 0xC000 | ((offset as u16) & 0x0FFF);
        Ok(())
    }

    fn brne(&mut self, target: usize, line: usize) -> Result<(), String> {
        let offset = target as isize - self.words.len() as isize - 1;
        if !(-64..=63).contains(&offset) {
            return Err(format!(
                "line {line}: repeat body is too large for this Uno backend"
            ));
        }
        self.emit(0xF401 | (((offset as u16) & 0x7F) << 3));
        Ok(())
    }

    fn ldi(&mut self, register: u16, value: u8) {
        let k = value as u16;
        self.emit(0xE000 | ((k & 0xF0) << 4) | ((register - 16) << 4) | (k & 0x0F));
    }

    fn lds(&mut self, register: u16, address: u16) {
        self.emit(0x9000 | (register << 4));
        self.emit(address);
    }

    fn sts(&mut self, address: u16, register: u16) {
        self.emit(0x9200 | (register << 4));
        self.emit(address);
    }

    fn store_word(&mut self, address: u16, low: u16) {
        self.sts(address, low);
        self.sts(address + 1, low + 1);
    }

    fn load(&mut self, binding: Binding, low: u16) {
        match binding {
            Binding::Const(number) => {
                self.ldi(low, number as u8);
                self.ldi(low + 1, (number >> 8) as u8);
            }
            Binding::Var(address) => {
                self.lds(low, address);
                self.lds(low + 1, address + 1);
            }
        }
    }

    fn block(
        &mut self,
        statements: &[Stmt],
        values: &HashMap<String, Binding>,
        depth: usize,
    ) -> Result<(), String> {
        for stmt in statements {
            match &stmt.kind {
                StmtKind::Pin(pin, mode) => {
                    let (ddr, port, _, bit) =
                        pin_registers(resolve(pin, values, stmt.line)?, stmt.line)?;
                    match mode.as_str() {
                        "out" => self.emit(sbi(ddr, bit)),
                        "in" => {
                            self.emit(cbi(ddr, bit));
                            self.emit(cbi(port, bit));
                        }
                        "pullup" => {
                            self.emit(cbi(ddr, bit));
                            self.emit(sbi(port, bit));
                        }
                        _ => {
                            return Err(format!(
                                "line {}: pin mode must be in, out, or pullup",
                                stmt.line
                            ));
                        }
                    }
                }
                StmtKind::On(pin) | StmtKind::Off(pin) => {
                    let number = resolve(pin, values, stmt.line)?;
                    let (_, port, _, bit) = pin_registers(number, stmt.line)?;
                    self.detach_pwm(number);
                    self.emit(if matches!(stmt.kind, StmtKind::On(_)) {
                        sbi(port, bit)
                    } else {
                        cbi(port, bit)
                    });
                }
                StmtKind::Wait(time) => {
                    let millis = value(time, values, stmt.line)?;
                    if !matches!(millis, Binding::Const(0)) {
                        self.wait(millis, stmt.line)?;
                    }
                }
                StmtKind::Power(pin, amount) => {
                    let number = resolve(pin, values, stmt.line)?;
                    let (compare_low, channel_bit, ddr_bit) = match number {
                        9 => (0x0088, 0x80, 1),
                        10 => (0x008A, 0x20, 2),
                        _ => {
                            return Err(format!(
                                "line {}: native PWM currently supports Uno pins 9 and 10",
                                stmt.line
                            ));
                        }
                    };
                    let amount = value(amount, values, stmt.line)?;
                    if matches!(amount, Binding::Const(n) if n > 255) {
                        return Err(format!(
                            "line {}: PWM power must be 0 through 255",
                            stmt.line
                        ));
                    }
                    self.emit(sbi(0x04, ddr_bit));
                    self.lds(24, 0x0080);
                    self.emit(imm_op(0x6000, 24, channel_bit)); // ORI: connect Timer1 output
                    self.sts(0x0080, 24);
                    self.load(amount, 24);
                    self.ldi(23, 0);
                    self.emit(reg_op(0x1400, 25, 23)); // CP high byte, zero
                    self.emit(brbs(1, 1)); // BREQ skips saturation when high byte is zero
                    self.ldi(24, 255);
                    self.sts(compare_low + 1, 23);
                    self.sts(compare_low, 24);
                }
                StmtKind::Read(pin, name) => {
                    let number = resolve(pin, values, stmt.line)?;
                    let (_, _, input, bit) = pin_registers(number, stmt.line)?;
                    let address = match values.get(name) {
                        Some(Binding::Var(address)) => *address,
                        _ => {
                            return Err(format!(
                                "line {}: read target {} is not writable",
                                stmt.line, name
                            ));
                        }
                    };
                    self.ldi(24, 0);
                    self.ldi(25, 0);
                    self.emit(0x9900 | (input << 3) | bit); // SBIC: skip next LDI when input is low
                    self.ldi(24, 1);
                    self.store_word(address, 24);
                }
                StmtKind::Math(name, expr) => {
                    let address = match values.get(name) {
                        Some(Binding::Var(address)) => *address,
                        _ => {
                            return Err(format!(
                                "line {}: math target {} needs a set or read declaration",
                                stmt.line, name
                            ));
                        }
                    };
                    let parts: Vec<_> = expr.split_whitespace().collect();
                    match parts.as_slice() {
                        [operand] => self.load(value(operand, values, stmt.line)?, 24),
                        [left, operator, right] if *operator == "+" || *operator == "-" => {
                            self.load(value(left, values, stmt.line)?, 24);
                            self.load(value(right, values, stmt.line)?, 22);
                            let (low, high) = if *operator == "+" {
                                (0x0C00, 0x1C00)
                            } else {
                                (0x1800, 0x0800)
                            };
                            self.emit(reg_op(low, 24, 22));
                            self.emit(reg_op(high, 25, 23));
                        }
                        _ => {
                            return Err(format!(
                                "line {}: math supports a value or value + or - value",
                                stmt.line
                            ));
                        }
                    }
                    self.store_word(address, 24);
                }
                StmtKind::Repeat(count, body) => {
                    let count = value(count, values, stmt.line)?;
                    if matches!(count, Binding::Const(0)) {
                        continue;
                    }
                    if depth >= 4 {
                        return Err(format!(
                            "line {}: at most four nested repeats are supported",
                            stmt.line
                        ));
                    }
                    let low = [16, 18, 20, 28][depth];
                    self.load(count, low);
                    let skip = if matches!(count, Binding::Var(_)) {
                        self.ldi(30, 0);
                        self.emit(reg_op(0x1400, low, 30));
                        self.emit(reg_op(0x0400, low + 1, 30));
                        self.emit(brbc(1, 1)); // BRNE skips the empty-count jump
                        let at = self.words.len();
                        self.emit(0);
                        Some(at)
                    } else {
                        None
                    };
                    let start = self.words.len();
                    self.block(body, values, depth + 1)?;
                    self.emit(imm_op(0x5000, low, 1)); // SUBI
                    self.emit(imm_op(0x4000, low + 1, 0)); // SBCI
                    self.brne(start, stmt.line)?;
                    if let Some(at) = skip {
                        self.patch_rjmp(at, self.words.len(), stmt.line)?;
                    }
                }
                StmtKind::If(condition, yes, no) => {
                    let parts: Vec<_> = condition.split_whitespace().collect();
                    if parts.len() != 3 {
                        return Err(format!("line {}: expected if value == value:", stmt.line));
                    }
                    let a = value(parts[0], values, stmt.line)?;
                    let b = value(parts[2], values, stmt.line)?;
                    if !matches!(parts[1], "==" | "!=" | ">" | "<" | ">=" | "<=") {
                        return Err(format!(
                            "line {}: unknown comparison {}",
                            stmt.line, parts[1]
                        ));
                    }
                    if let (Binding::Const(a), Binding::Const(b)) = (a, b) {
                        let result = match parts[1] {
                            "==" => a == b,
                            "!=" => a != b,
                            ">" => a > b,
                            "<" => a < b,
                            ">=" => a >= b,
                            "<=" => a <= b,
                            _ => unreachable!(),
                        };
                        self.block(if result { yes } else { no }, values, depth)?;
                    } else {
                        self.load(a, 24);
                        self.load(b, 22);
                        self.emit(reg_op(0x1400, 24, 22)); // CP low
                        self.emit(reg_op(0x0400, 25, 23)); // CPC high
                        match parts[1] {
                            "==" => self.emit(brbs(1, 1)),
                            "!=" => self.emit(brbc(1, 1)),
                            "<" => self.emit(brbs(0, 1)),
                            ">=" => self.emit(brbc(0, 1)),
                            ">" => {
                                self.emit(brbs(0, 1)); // carry means below
                                self.emit(brbc(1, 1)); // nonzero and no carry means above
                            }
                            "<=" => {
                                self.emit(brbs(0, 2)); // below
                                self.emit(brbs(1, 1)); // equal
                            }
                            _ => unreachable!(),
                        }
                        let false_jump = self.words.len();
                        self.emit(0);
                        self.block(yes, values, depth)?;
                        if no.is_empty() {
                            self.patch_rjmp(false_jump, self.words.len(), stmt.line)?;
                        } else {
                            let end_jump = self.words.len();
                            self.emit(0);
                            self.patch_rjmp(false_jump, self.words.len(), stmt.line)?;
                            self.block(no, values, depth)?;
                            self.patch_rjmp(end_jump, self.words.len(), stmt.line)?;
                        }
                    }
                }
                StmtKind::Call(name, args) => {
                    let task = self
                        .tasks
                        .get(name.as_str())
                        .copied()
                        .ok_or_else(|| format!("line {}: unknown task {}", stmt.line, name))?;
                    if args.len() != task.params.len() {
                        return Err(format!(
                            "line {}: task {} expects {} arguments",
                            stmt.line,
                            name,
                            task.params.len()
                        ));
                    }
                    if self.call_stack.iter().any(|n| n == name) {
                        return Err(format!(
                            "line {}: recursive tasks are not supported",
                            stmt.line
                        ));
                    }
                    let mut local = values.clone();
                    for (param, arg) in task.params.iter().zip(args) {
                        local.insert(param.clone(), value(arg, values, stmt.line)?);
                    }
                    self.call_stack.push(name.clone());
                    let result = self.block(&task.body, &local, depth);
                    self.call_stack.pop();
                    result?;
                }
                StmtKind::Unsupported(text) => {
                    return Err(format!(
                        "line {}: unsupported Uno native statement: {}",
                        stmt.line, text
                    ));
                }
            }
        }
        Ok(())
    }

    fn detach_pwm(&mut self, pin: u16) {
        if !self.uses_pwm || !matches!(pin, 9 | 10) {
            return;
        }
        let mask = if pin == 9 { 0x7F } else { 0xDF };
        self.lds(24, 0x0080);
        self.emit(imm_op(0x7000, 24, mask)); // ANDI: disconnect Timer1 output
        self.sts(0x0080, 24);
    }

    fn wait(&mut self, millis: Binding, line: usize) -> Result<(), String> {
        // 4000 iterations of SBIW + BRNE take about 16000 cycles at 16 MHz.
        self.load(millis, 26);
        let skip = if matches!(millis, Binding::Var(_)) {
            self.ldi(30, 0);
            self.emit(reg_op(0x1400, 26, 30));
            self.emit(reg_op(0x0400, 27, 30));
            self.emit(brbc(1, 1)); // BRNE skips the zero-delay jump
            let at = self.words.len();
            self.emit(0);
            Some(at)
        } else {
            None
        };
        let outer = self.words.len();
        self.ldi(24, 4000u16 as u8);
        self.ldi(25, (4000u16 >> 8) as u8);
        let inner = self.words.len();
        self.emit(0x9701); // SBIW r25:r24, 1
        self.brne(inner, line)?;
        self.emit(0x9711); // SBIW r27:r26, 1
        self.brne(outer, line)?;
        if let Some(at) = skip {
            self.patch_rjmp(at, self.words.len(), line)?;
        }
        Ok(())
    }
}

fn reg_op(base: u16, dest: u16, source: u16) -> u16 {
    base | ((source & 0x10) << 5) | (dest << 4) | (source & 0x0F)
}

fn imm_op(base: u16, register: u16, value: u8) -> u16 {
    let k = value as u16;
    base | ((k & 0xF0) << 4) | ((register - 16) << 4) | (k & 0x0F)
}

fn brbs(bit: u16, offset: u16) -> u16 {
    0xF000 | (offset << 3) | bit
}
fn brbc(bit: u16, offset: u16) -> u16 {
    0xF400 | (offset << 3) | bit
}

fn pin_registers(pin: u16, line: usize) -> Result<(u16, u16, u16, u16), String> {
    match pin {
        0..=7 => Ok((0x0A, 0x0B, 0x09, pin)),
        8..=13 => Ok((0x04, 0x05, 0x03, pin - 8)),
        _ => Err(format!("line {line}: Uno digital pin must be 0 through 13")),
    }
}

fn sbi(io: u16, bit: u16) -> u16 {
    0x9A00 | (io << 3) | bit
}
fn cbi(io: u16, bit: u16) -> u16 {
    0x9800 | (io << 3) | bit
}

pub fn intel_hex(bytes: &[u8]) -> String {
    let mut out = String::new();
    for (i, chunk) in bytes.chunks(16).enumerate() {
        let address = i * 16;
        let mut sum = chunk.len() as u8;
        sum = sum
            .wrapping_add((address >> 8) as u8)
            .wrapping_add(address as u8);
        out.push_str(&format!(":{:02X}{:04X}00", chunk.len(), address));
        for byte in chunk {
            out.push_str(&format!("{byte:02X}"));
            sum = sum.wrapping_add(*byte);
        }
        out.push_str(&format!("{:02X}\n", (!sum).wrapping_add(1)));
    }
    out.push_str(":00000001FF\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer, parser::Parser};

    fn compile_source(source: &str) -> Result<Vec<u8>, String> {
        let lines = lexer::tokenize(source)?;
        let program = Parser::new(lines).parse_program()?;
        compile(&program)
    }

    #[test]
    fn blink_emits_real_avr_words() {
        let bytes =
            compile_source("set led 13\nsetup:\n    pin led out\nloop:\n    on led\n    off led\n")
                .unwrap();
        let words: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        assert_eq!(words, vec![0x9A25, 0x9A2D, 0x982D, 0xCFFD]);
    }

    #[test]
    fn tasks_and_repeat_compile() {
        let bytes = compile_source("task pulse(pin):\n    repeat 2:\n        on pin\n        off pin\nloop:\n    pulse(13)\n").unwrap();
        assert!(bytes.len() > 8);
    }

    #[test]
    fn unsupported_pwm_pin_is_error() {
        let err = compile_source("loop:\n    power 3 128\n").unwrap_err();
        assert!(err.contains("line 2: native PWM currently supports Uno pins 9 and 10"));
    }

    #[test]
    fn constant_condition_selects_else() {
        let bytes =
            compile_source("loop:\n    if 1 == 2:\n        on 13\n    else:\n        off 13\n")
                .unwrap();
        let words: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        assert_eq!(words, vec![0x982D, 0xCFFE]);
    }

    #[test]
    fn invalid_indent_is_reported() {
        let err = compile_source("loop:\n    on 13\n      off 13\n").unwrap_err();
        assert!(err.contains("line 3: unexpected indentation"));
    }

    #[test]
    fn out_of_range_value_is_reported() {
        let err = compile_source("set led 70000\nloop:\n    on led\n").unwrap_err();
        assert!(err.contains("line 1: value must be 0 through 65535"));
    }

    #[test]
    fn mutable_math_and_runtime_condition_compile() {
        let source = "set speed 0\nloop:\n    math speed = speed + 50\n    if speed > 200:\n        power 9 speed\n        math speed = 0\n";
        let bytes = compile_source(source).unwrap();
        let words: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        assert!(words.contains(&0x0F86)); // ADD r24, r22
        assert!(words.contains(&0x1F97)); // ADC r25, r23
        assert!(words.contains(&0x0080)); // Timer1 control register address
        assert!(words.contains(&0x0088)); // OCR1AL address
    }

    #[test]
    fn digital_read_emits_pin_test() {
        let bytes =
            compile_source("loop:\n    read 2 into pressed\n    if pressed == 1:\n        on 13\n")
                .unwrap();
        let words: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        assert!(words.contains(&0x994A)); // SBIC PIND, 2
    }

    #[test]
    fn runtime_wait_and_repeat_compile() {
        let source = "set delay_ms 2\nset times 2\nloop:\n    math times = times - 1\n    repeat times:\n        wait delay_ms\n    math delay_ms = 1\n";
        let bytes = compile_source(source).unwrap();
        let words: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        assert!(words.contains(&0x9100)); // LDS r16, variable low byte
        assert!(words.contains(&0x91A0)); // LDS r26, delay low byte
    }

    #[test]
    fn pullup_configures_input_and_resistor() {
        let bytes =
            compile_source("setup:\n    pin 2 pullup\nloop:\n    read 2 into pressed\n").unwrap();
        let words: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        assert!(words.contains(&0x9852)); // CBI DDRD, 2
        assert!(words.contains(&0x9A5A)); // SBI PORTD, 2
    }

    #[test]
    fn hex_checksum_is_valid() {
        let hex = intel_hex(&[0x25, 0x9A]);
        assert_eq!(hex, ":02000000259A3F\n:00000001FF\n");
    }
}
