use crate::lexer::Line;

#[derive(Debug, Clone)]
pub struct Program {
    pub constants: Vec<(String, String, usize)>,
    pub tasks: Vec<Task>,
    pub setup: Vec<Stmt>,
    pub loop_body: Vec<Stmt>,
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
    Math(String, String),
    Repeat(String, Vec<Stmt>),
    If(String, Vec<Stmt>, Vec<Stmt>),
    Call(String, Vec<String>),
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
            constants: Vec::new(),
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
            if let Some(rest) = line.text.strip_prefix("set ") {
                let words: Vec<_> = rest.split_whitespace().collect();
                if words.len() != 2 || !valid_name(words[0]) {
                    return Err(format!("line {}: expected set name value", line.number));
                }
                program
                    .constants
                    .push((words[0].into(), words[1].into(), line.number));
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
                    "line {}: expected set, task, setup:, or loop:",
                    line.number
                ));
            }
        }
        if !has_loop {
            return Err("missing loop: block".into());
        }
        Ok(program)
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
            let kind = if let Some(count) = line
                .text
                .strip_prefix("repeat ")
                .and_then(|s| s.strip_suffix(':'))
            {
                StmtKind::Repeat(count.trim().into(), self.parse_block(indent)?)
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
    let args: Vec<String> = if args.trim().is_empty() {
        Vec::new()
    } else {
        args.split(',').map(|s| s.trim().to_string()).collect()
    };
    if args
        .iter()
        .any(|a| a.is_empty() || (task && !valid_name(a)))
    {
        return Err(format!("line {line}: invalid argument list"));
    }
    Ok((name.into(), args))
}
