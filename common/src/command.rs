use crate::types::CommandID;

pub trait HeadlightCommand {
    const ID: CommandID;
}
