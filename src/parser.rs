use crate::lexer::Token;

// --- 🌳 THE ABSTRACT SYNTAX TREE (AST) ---

#[derive(Debug, Clone)]
pub struct Program {
    pub tasks: Vec<Task>,
    pub main_loop: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Set(String, String),
    PinMode(String, String),
    TurnOn(String),
    TurnOff(String),
    Wait(String),
    TaskCall(String, Vec<String>),
    // 🚀 NEW: Restoring V1 Features to the AST
    Power(String, String),
    Read(String, String),
    MathOp(String, String),
    RepeatBlock(String, Vec<Stmt>),
    IfBlock(String, Vec<Stmt>, Vec<Stmt>), // condition, if-body, else-body
}

// --- 🧠 THE PARSER LOGIC ---

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> { self.tokens.get(self.pos) }
    fn advance(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.pos);
        self.pos += 1;
        token
    }

    pub fn parse_program(&mut self) -> Program {
        let mut program = Program { tasks: Vec::new(), main_loop: Vec::new() };

        while let Some(token) = self.peek() {
            match token {
                Token::Task => program.tasks.push(self.parse_task()),
                Token::Loop => {
                    self.advance();
                    program.main_loop = self.parse_block(1); // Main loop base indent is 1
                }
                Token::Indent(_) => { self.advance(); }
                Token::Set(name, val) => {
                    program.main_loop.push(Stmt::Set(name.clone(), val.clone()));
                    self.advance();
                }
                _ => { self.advance(); }
            }
        }
        program
    }

    fn parse_task(&mut self) -> Task {
        self.advance(); // consume 'task'
        let mut name = String::new();
        if let Some(Token::Identifier(n)) = self.advance() { name = n.clone(); }
        
        self.advance(); // consume '('
        let mut params = Vec::new();
        while let Some(token) = self.peek() {
            match token {
                Token::Identifier(param) => { params.push(param.clone()); self.advance(); }
                Token::Comma => { self.advance(); }
                Token::RParen => { self.advance(); break; }
                _ => { self.advance(); }
            }
        }
        
        let body = self.parse_block(1); // Task body base indent is 1
        Task { name, params, body }
    }

    // Parses a block of statements based on indentation depth
    fn parse_block(&mut self, expected_indent: usize) -> Vec<Stmt> {
        let mut statements = Vec::new();

        while let Some(token) = self.peek() {
            match token {
                Token::Indent(level) => {
                    if *level < expected_indent { break; } // Block is over!
                    self.advance();
                }
                Token::On(pin) => { statements.push(Stmt::TurnOn(pin.clone())); self.advance(); }
                Token::Off(pin) => { statements.push(Stmt::TurnOff(pin.clone())); self.advance(); }
                Token::Wait(time) => { statements.push(Stmt::Wait(time.clone())); self.advance(); }
                Token::Power(pin, val) => { statements.push(Stmt::Power(pin.clone(), val.clone())); self.advance(); }
                Token::Read(pin, var) => { statements.push(Stmt::Read(pin.clone(), var.clone())); self.advance(); }
                Token::Math(var, expr) => { statements.push(Stmt::MathOp(var.clone(), expr.clone())); self.advance(); }
                
                // Nested Blocks (Recursion!)
                Token::Repeat(times) => {
                    let t = times.clone();
                    self.advance();
                    let body = self.parse_block(expected_indent + 1);
                    statements.push(Stmt::RepeatBlock(t, body));
                }
                Token::If(condition) => {
                    let cond = condition.clone();
                    self.advance();
                    let if_body = self.parse_block(expected_indent + 1);
                    let mut else_body = Vec::new();
                    
                    // Check if an 'else' follows immediately
                    if let Some(Token::Else) = self.peek() {
                        self.advance();
                        else_body = self.parse_block(expected_indent + 1);
                    }
                    statements.push(Stmt::IfBlock(cond, if_body, else_body));
                }
                
                Token::Identifier(name) => {
                    let task_name = name.clone();
                    self.advance();
                    if let Some(Token::LParen) = self.peek() {
                        self.advance();
                        let mut args = Vec::new();
                        while let Some(arg_token) = self.peek() {
                            match arg_token {
                                Token::Identifier(val) | Token::Number(val) => { args.push(val.clone()); self.advance(); }
                                Token::Comma => { self.advance(); }
                                Token::RParen => { self.advance(); break; }
                                _ => { self.advance(); }
                            }
                        }
                        statements.push(Stmt::TaskCall(task_name, args));
                    }
                }
                Token::Task | Token::Loop => break, 
                _ => { self.advance(); }
            }
        }
        statements
    }
}