use crate::lexer::Line;

#[derive(Debug, Clone)]
pub struct Program {
    pub board: Option<String>,
    pub cpp_global: Vec<String>,
    pub arduino_sketch: Option<String>,
    pub libraries: Vec<(String, usize)>,
    pub devices: Vec<DeviceSpec>,
    pub constants: Vec<(String, String, usize)>,
    pub vectors: Vec<VectorSpec>,
    pub models: Vec<ModelSpec>,
    pub tasks: Vec<Task>,
    pub setup: Vec<Stmt>,
    pub loop_body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct DeviceSpec {
    pub name: String,
    pub kind: String,
    pub args: Vec<String>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct VectorSpec {
    pub name: String,
    pub length: usize,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct ModelSpec {
    pub name: String,
    pub path: String,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct Stmt {
    pub line: usize,
    pub kind: StmtKind,
}

#[derive(Debug, Clone)]
pub enum StmtKind {
    Pin(String, String),
    On(String),
    Off(String),
    Wait(String),
    Power(String, String),
    Read(String, String),
    ReadAnalog(String, String),
    ReadImu(String, String, String),
    Time(String),
    Elapsed(String, String),
    VectorGet(String, String, String),
    VectorPut(String, String, String),
    Dot(String, String, String),
    Predict(String, String, String),
    Math(String, String),
    Repeat(String, Vec<Stmt>),
    While(String, Vec<Stmt>),
    If(String, Vec<Stmt>, Vec<Stmt>),
    Call(String, Vec<String>),
    ArduinoCall(String, Vec<String>, Option<String>),
    Cpp(String),
    Unsupported(String),
}

pub struct Parser {
    lines: Vec<Line>,
    pos: usize,
}

impl Parser {
    pub fn new(lines: Vec<Line>) -> Self {
        Self { lines, pos: 0 }
    }

    pub fn parse_program(&mut self) -> Result<Program, String> {
        let mut program = Program {
            board: None,
            cpp_global: Vec::new(),
            arduino_sketch: None,
            libraries: Vec::new(),
            devices: Vec::new(),
            constants: Vec::new(),
            vectors: Vec::new(),
            models: Vec::new(),
            tasks: Vec::new(),
            setup: Vec::new(),
            loop_body: Vec::new(),
        };
        let mut has_setup = false;
        let mut has_loop = false;
        while self.pos < self.lines.len() {
            let line = self.lines[self.pos].clone();
            if line.indent != 0 {
                return Err(format!(
                    "line {}: top-level statement must start at column 1",
                    line.number
                ));
            }
            if let Some(board) = line.text.strip_prefix("board ") {
                if program.board.is_some()
                    || !matches!(board, "nano33blesense_rev2" | "uno_r4_minima" | "uno_r3")
                {
                    return Err(format!(
                        "line {}: expected board uno_r3, uno_r4_minima, or nano33blesense_rev2 (once)",
                        line.number
                    ));
                }
                program.board = Some(board.into());
                self.pos += 1;
            } else if line.text == "cpp:" {
                self.pos += 1;
                program.cpp_global.push(self.parse_raw_block(0)?);
            } else if let Some(header) = line.text.strip_prefix("use ") {
                let header = header.trim().trim_matches('"');
                if header.is_empty()
                    || !header
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/'))
                {
                    return Err(format!("line {}: expected use \"Library.h\"", line.number));
                }
                program.libraries.push((header.into(), line.number));
                self.pos += 1;
            } else if let Some(rest) = line.text.strip_prefix("device ") {
                let (name, constructor) = rest.split_once('=').ok_or_else(|| {
                    format!(
                        "line {}: expected device name = Type(arguments)",
                        line.number
                    )
                })?;
                let name = name.trim();
                let (kind, args) = parse_call_header(constructor.trim(), line.number, false)?;
                if !valid_name(name) {
                    return Err(format!("line {}: invalid device name {name}", line.number));
                }
                program.devices.push(DeviceSpec {
                    name: name.into(),
                    kind,
                    args,
                    line: line.number,
                });
                self.pos += 1;
            } else if line.text == "arduino:" {
                if program.arduino_sketch.is_some() {
                    return Err(format!("line {}: duplicate arduino: block", line.number));
                }
                self.pos += 1;
                program.arduino_sketch = Some(self.parse_raw_block(0)?);
            } else if let Some(rest) = line.text.strip_prefix("vector ") {
                let words: Vec<_> = rest.split_whitespace().collect();
                if words.len() != 2 || !valid_name(words[0]) {
                    return Err(format!("line {}: expected vector name length", line.number));
                }
                let length = words[1].parse::<usize>().map_err(|_| {
                    format!("line {}: vector length must be 1 through 1024", line.number)
                })?;
                if !(1..=1024).contains(&length) {
                    return Err(format!(
                        "line {}: vector length must be 1 through 1024",
                        line.number
                    ));
                }
                program.vectors.push(VectorSpec {
                    name: words[0].into(),
                    length,
                    line: line.number,
                });
                self.pos += 1;
            } else if let Some(rest) = line.text.strip_prefix("set ") {
                let words: Vec<_> = rest.split_whitespace().collect();
                if words.len() != 2 || !valid_name(words[0]) {
                    return Err(format!("line {}: expected set name value", line.number));
                }
                program
                    .constants
                    .push((words[0].into(), words[1].into(), line.number));
                self.pos += 1;
            } else if let Some(rest) = line.text.strip_prefix("learn ") {
                let (name, path) = rest.split_once(" from ").ok_or_else(|| {
                    format!(
                        "line {}: expected learn name from \"file.csv\"",
                        line.number
                    )
                })?;
                let path = path
                    .trim()
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'));
                if !valid_name(name) || path.is_none_or(str::is_empty) {
                    return Err(format!(
                        "line {}: expected learn name from \"file.csv\"",
                        line.number
                    ));
                }
                program.models.push(ModelSpec {
                    name: name.into(),
                    path: path.unwrap().into(),
                    line: line.number,
                });
                self.pos += 1;
            } else if line.text == "setup:" {
                if has_setup {
                    return Err(format!("line {}: duplicate setup block", line.number));
                }
                has_setup = true;
                self.pos += 1;
                program.setup = self.parse_block(0)?;
            } else if line.text == "loop:" {
                if has_loop {
                    return Err(format!("line {}: duplicate loop block", line.number));
                }
                has_loop = true;
                self.pos += 1;
                program.loop_body = self.parse_block(0)?;
            } else if let Some(header) = line.text.strip_prefix("task ") {
                let (name, params) = parse_call_header(header, line.number, true)?;
                self.pos += 1;
                let body = self.parse_block(0)?;
                program.tasks.push(Task { name, params, body });
            } else {
                return Err(format!(
                    "line {}: expected board, use, device, cpp:, arduino:, vector, set, learn, task, setup:, or loop:",
                    line.number
                ));
            }
        }
        if program.arduino_sketch.is_some() {
            if program.board.is_none() {
                return Err("arduino: requires a board declaration".into());
            }
            if has_setup
                || has_loop
                || !program.tasks.is_empty()
                || !program.constants.is_empty()
                || !program.vectors.is_empty()
                || !program.models.is_empty()
                || !program.cpp_global.is_empty()
                || !program.libraries.is_empty()
                || !program.devices.is_empty()
            {
                return Err(
                    "arduino: is a complete sketch; it cannot be mixed with ET blocks".into(),
                );
            }
        } else if !has_loop {
            return Err("missing loop: block".into());
        }
        if program.board.is_none()
            && (!program.cpp_global.is_empty()
                || !program.libraries.is_empty()
                || !program.devices.is_empty())
        {
            return Err("cpp:, use, and device require an explicit Arduino board".into());
        }
        Ok(program)
    }

    fn parse_raw_block(&mut self, parent_indent: usize) -> Result<String, String> {
        let start = self.pos;
        let first = self.lines[start..]
            .iter()
            .find(|line| !line.text.is_empty())
            .ok_or_else(|| {
                format!(
                    "line {}: expected an indented C++ block",
                    self.lines[start - 1].number
                )
            })?;
        if first.indent <= parent_indent {
            return Err(format!(
                "line {}: expected an indented C++ block",
                self.lines[start - 1].number
            ));
        }
        let base_indent = first.raw.len() - first.raw.trim_start_matches([' ', '\t']).len();
        let mut out = String::new();
        while self.pos < self.lines.len() {
            let line = &self.lines[self.pos];
            if !line.text.is_empty() && line.indent <= parent_indent {
                break;
            }
            if line.text.is_empty() {
                out.push('\n');
            } else {
                let prefix_len = line.raw.len() - line.raw.trim_start_matches([' ', '\t']).len();
                let removed = prefix_len.min(base_indent);
                out.push_str(&line.raw[removed..]);
                out.push('\n');
            }
            self.pos += 1;
        }
        Ok(out)
    }

    fn parse_block(&mut self, parent_indent: usize) -> Result<Vec<Stmt>, String> {
        if self.pos >= self.lines.len() || self.lines[self.pos].indent <= parent_indent {
            return Err(format!(
                "line {}: expected an indented block",
                self.lines[self.pos.saturating_sub(1)].number
            ));
        }
        let indent = self.lines[self.pos].indent;
        let mut statements = Vec::new();
        while self.pos < self.lines.len() && self.lines[self.pos].indent >= indent {
            let line = self.lines[self.pos].clone();
            if line.indent != indent {
                return Err(format!("line {}: unexpected indentation", line.number));
            }
            self.pos += 1;
            let kind = if line.text == "cpp:" {
                StmtKind::Cpp(self.parse_raw_block(indent)?)
            } else if let Some(count) = line
                .text
                .strip_prefix("repeat ")
                .and_then(|s| s.strip_suffix(':'))
            {
                StmtKind::Repeat(count.trim().into(), self.parse_block(indent)?)
            } else if let Some(condition) = line
                .text
                .strip_prefix("while ")
                .and_then(|s| s.strip_suffix(':'))
            {
                StmtKind::While(condition.trim().into(), self.parse_block(indent)?)
            } else if let Some(condition) = line
                .text
                .strip_prefix("if ")
                .and_then(|s| s.strip_suffix(':'))
            {
                let yes = self.parse_block(indent)?;
                let no = if self.pos < self.lines.len()
                    && self.lines[self.pos].indent == indent
                    && self.lines[self.pos].text == "else:"
                {
                    self.pos += 1;
                    self.parse_block(indent)?
                } else {
                    Vec::new()
                };
                StmtKind::If(condition.trim().into(), yes, no)
            } else {
                let words: Vec<_> = line.text.split_whitespace().collect();
                match words.as_slice() {
                    ["pin", pin, mode] => StmtKind::Pin((*pin).into(), (*mode).into()),
                    ["on", pin] => StmtKind::On((*pin).into()),
                    ["off", pin] => StmtKind::Off((*pin).into()),
                    ["wait", time] => StmtKind::Wait((*time).into()),
                    ["power", pin, value] => StmtKind::Power((*pin).into(), (*value).into()),
                    ["read", pin, "into", name] if valid_name(name) => {
                        StmtKind::Read((*pin).into(), (*name).into())
                    }
                    ["read", "analog", pin, "into", name] if valid_name(name) => {
                        StmtKind::ReadAnalog((*pin).into(), (*name).into())
                    }
                    ["read", "imu", "into", x, y, z]
                        if valid_name(x) && valid_name(y) && valid_name(z) =>
                    {
                        StmtKind::ReadImu((*x).into(), (*y).into(), (*z).into())
                    }
                    ["time", "into", name] if valid_name(name) => StmtKind::Time((*name).into()),
                    ["elapsed", start, "into", name] if valid_name(name) => {
                        StmtKind::Elapsed((*start).into(), (*name).into())
                    }
                    ["get", vector, index, "into", name]
                        if valid_name(vector) && valid_name(name) =>
                    {
                        StmtKind::VectorGet((*vector).into(), (*index).into(), (*name).into())
                    }
                    ["put", vector, index, value] if valid_name(vector) => {
                        StmtKind::VectorPut((*vector).into(), (*index).into(), (*value).into())
                    }
                    ["dot", left, right, "into", name]
                        if valid_name(left) && valid_name(right) && valid_name(name) =>
                    {
                        StmtKind::Dot((*left).into(), (*right).into(), (*name).into())
                    }
                    ["predict", model, "from", input, "into", name]
                        if valid_name(model) && valid_name(name) =>
                    {
                        StmtKind::Predict((*model).into(), (*input).into(), (*name).into())
                    }
                    _ if line.text.starts_with("call ") => {
                        let rest = line.text.strip_prefix("call ").unwrap();
                        let end = rest.rfind(')').ok_or_else(|| {
                            format!("line {}: expected call name(arguments)", line.number)
                        })?;
                        let call = rest[..=end].trim();
                        let tail = rest[end + 1..].trim();
                        let output = if tail.is_empty() {
                            None
                        } else if let Some(name) = tail.strip_prefix("into ") {
                            if !valid_name(name.trim()) {
                                return Err(format!("line {}: invalid call output", line.number));
                            }
                            Some(name.trim().to_string())
                        } else {
                            return Err(format!("line {}: invalid call output", line.number));
                        };
                        let (path, args) = parse_arduino_call(call, line.number)?;
                        StmtKind::ArduinoCall(path, args, output)
                    }
                    _ if line.text.starts_with("math ") => {
                        let rest = line.text.strip_prefix("math ").unwrap();
                        let (name, expr) = rest.split_once('=').ok_or_else(|| {
                            format!("line {}: expected math name = expression", line.number)
                        })?;
                        let name = name.trim();
                        let expr = expr.trim();
                        if !valid_name(name) || expr.is_empty() {
                            return Err(format!(
                                "line {}: expected math name = expression",
                                line.number
                            ));
                        }
                        StmtKind::Math(name.into(), expr.into())
                    }
                    _ if line.text.ends_with(')') => {
                        let (name, args) = parse_call_header(&line.text, line.number, false)?;
                        StmtKind::Call(name, args)
                    }
                    _ => StmtKind::Unsupported(line.text.clone()),
                }
            };
            statements.push(Stmt {
                line: line.number,
                kind,
            });
        }
        Ok(statements)
    }
}

fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn parse_call_header(text: &str, line: usize, task: bool) -> Result<(String, Vec<String>), String> {
    let text = if task {
        text.strip_suffix(':')
            .ok_or_else(|| format!("line {line}: task header needs a colon"))?
    } else {
        text
    };
    let (name, rest) = text
        .split_once('(')
        .ok_or_else(|| format!("line {line}: expected name(arguments)"))?;
    let args = rest
        .strip_suffix(')')
        .ok_or_else(|| format!("line {line}: expected closing parenthesis"))?;
    let name = name.trim();
    if !valid_name(name) {
        return Err(format!("line {line}: invalid task name {name}"));
    }
    let args = split_arguments(args, line)?;
    if args
        .iter()
        .any(|a| a.is_empty() || (task && !valid_name(a)))
    {
        return Err(format!("line {line}: invalid argument list"));
    }
    Ok((name.into(), args))
}

fn parse_arduino_call(text: &str, line: usize) -> Result<(String, Vec<String>), String> {
    let (path, rest) = text
        .split_once('(')
        .ok_or_else(|| format!("line {line}: expected call name(arguments)"))?;
    let args = rest
        .strip_suffix(')')
        .ok_or_else(|| format!("line {line}: expected closing parenthesis"))?;
    let path = path.trim();
    if path.split('.').any(|part| !valid_name(part)) {
        return Err(format!("line {line}: invalid call name {path}"));
    }
    Ok((path.into(), split_arguments(args, line)?))
}

fn split_arguments(args: &str, line: usize) -> Result<Vec<String>, String> {
    if args.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut parts = Vec::new();
    let mut quote = None;
    let mut escaped = false;
    let mut start = 0;
    for (index, ch) in args.char_indices() {
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == delimiter {
                quote = None;
            }
        } else if ch == '"' || ch == '\'' {
            quote = Some(ch);
        } else if ch == ',' {
            let part = args[start..index].trim();
            if part.is_empty() {
                return Err(format!("line {line}: empty argument"));
            }
            parts.push(part.into());
            start = index + 1;
        }
    }
    if quote.is_some() {
        return Err(format!("line {line}: unterminated string argument"));
    }
    let part = args[start..].trim();
    if part.is_empty() {
        return Err(format!("line {line}: empty argument"));
    }
    parts.push(part.into());
    Ok(parts)
}
