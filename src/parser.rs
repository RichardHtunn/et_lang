use crate::lexer::Token;

// --- 🌳 THE ABSTRACT SYNTAX TREE (AST) DATA STRUCTURES ---

#[derive(Debug, Clone)]
pub struct Program {
    pub tasks: Vec<Task>,        // Stores all your custom functions
    pub main_loop: Vec<Stmt>,    // Stores the code inside 'loop:'
}

#[derive(Debug, Clone)]
pub struct Task {
    pub name: String,
    pub params: Vec<String>,     // E.g., ["target_pin", "delay_time"]
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Set(String, String),
    PinMode(String, String),
    TurnOn(String),
    TurnOff(String),
    Wait(String),
    TaskCall(String, Vec<String>), // E.g., flash("13", "500")
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

    // Peeks at the current token without consuming it
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    // Consumes the current token and moves forward
    fn advance(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.pos);
        self.pos += 1;
        token
    }

    // The main entry point: Parses the entire file into a 'Program'
    pub fn parse_program(&mut self) -> Program {
        let mut program = Program {
            tasks: Vec::new(),
            main_loop: Vec::new(),
        };

        while let Some(token) = self.peek() {
            match token {
                // If we see the 'task' keyword, parse a function!
                Token::Task => {
                    program.tasks.push(self.parse_task());
                }
                // If we see 'loop:', parse the main execution block!
                Token::Loop => {
                    self.advance(); // consume 'loop:'
                    program.main_loop = self.parse_block();
                }
                // Skip indentation tokens at the root level
                Token::Indent(_) => {
                    self.advance();
                }
                // For V1 compatibility, handle global sets
                Token::Set(name, val) => {
                    let s = Stmt::Set(name.clone(), val.clone());
                    program.main_loop.push(s); // Hack for now: put globals in main loop
                    self.advance();
                }
                _ => {
                    self.advance(); // Ignore unknowns for now to prevent infinite loops
                }
            }
        }
        program
    }

    // Parses a single task: task name(arg1, arg2):
    fn parse_task(&mut self) -> Task {
        self.advance(); // consume 'task' keyword

        let mut name = String::new();
        if let Some(Token::Identifier(n)) = self.advance() {
            name = n.clone();
        }

        self.advance(); // consume '('

        let mut params = Vec::new();
        // Keep reading arguments until we hit ')'
        while let Some(token) = self.peek() {
            match token {
                Token::Identifier(param) => {
                    params.push(param.clone());
                    self.advance();
                }
                Token::Comma => { self.advance(); } // just skip commas
                Token::RParen => {
                    self.advance(); // consume ')'
                    break;
                }
                _ => { self.advance(); }
            }
        }

        // Now parse the indented code underneath the task declaration
        let body = self.parse_block();

        Task { name, params, body }
    }

    // Parses a block of indented statements
    fn parse_block(&mut self) -> Vec<Stmt> {
        let mut statements = Vec::new();

        while let Some(token) = self.peek() {
            match token {
                Token::On(pin) => {
                    statements.push(Stmt::TurnOn(pin.clone()));
                    self.advance();
                }
                Token::Off(pin) => {
                    statements.push(Stmt::TurnOff(pin.clone()));
                    self.advance();
                }
                Token::Wait(time) => {
                    statements.push(Stmt::Wait(time.clone()));
                    self.advance();
                }
                // Recognize a Task Call (e.g., flash(13, 500))
                Token::Identifier(name) => {
                    let task_name = name.clone();
                    self.advance(); // consume name

                    if let Some(Token::LParen) = self.peek() {
                        self.advance(); // consume '('
                        let mut args = Vec::new();