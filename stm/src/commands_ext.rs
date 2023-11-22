pub enum CommandExecutionError {
    
}

pub trait Execute {
    fn run(self) -> Result<(), CommandExecutionError>;
}

