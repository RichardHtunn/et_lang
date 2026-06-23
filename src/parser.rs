pub struct Program {
    pub tasks: Vec<Task>,    
    pub main_loop: Vec<Stmt>,  
}

pub struct Task {
    pub name: String,           
    pub params: Vec<String>,     
    pub body: Vec<Stmt>,        
}

pub enum Stmt {
    TurnOn(String),                          
    TurnOff(String),                           
    Wait(String),                                
    TaskCall(String, Vec<String>),                 
}