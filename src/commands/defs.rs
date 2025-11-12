use std::io;

use crate::commands::handlers::ping;
// TODO: Import get handler when implemented
// use crate::commands::handlers::get;
// TODO: Import set handler when implemented
// use crate::commands::handlers::set;
use crate::commands::parser::{ArgumentError, ArgumentParser, ParsedArguments};

#[derive(Clone)]
pub enum CommandType {
    Ping,
    Get,
    Set,
}

pub trait CommandHandler {
    fn name(&self) -> &'static str;

    fn parser(&self) -> ArgumentParser {
        ArgumentParser::new(self.name(), vec![])
    }

    fn execute(&self, args: &ParsedArguments) -> Result<String, ArgumentError>;

    fn handle(&self, args: &[&str]) -> Result<String, ArgumentError> {
        let parser = self.parser();
        let parsed = parser.parse(args)?;
        self.execute(&parsed)
    }
}

static PING_HANDLER: ping::PingHandler = ping::PingHandler;
// TODO: Implement GET_HANDLER
// static GET_HANDLER: get::GetHandler = get::GetHandler;
// TODO: Implement SET_HANDLER
// static SET_HANDLER: set::SetHandler = set::SetHandler;

pub fn match_command(input: &str) -> io::Result<CommandType> {
    match input.trim().to_lowercase().as_str() {
        "ping" => Ok(CommandType::Ping),
        "get" => Ok(CommandType::Get),
        "set" => Ok(CommandType::Set),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Unknown command: {}", input.trim()),
        )),
    }
}

fn get_handler_for(cmd: &CommandType) -> &'static dyn CommandHandler {
    match cmd {
        CommandType::Ping => &PING_HANDLER,
        // TODO: Implement Get handler
        CommandType::Get => todo!("Get handler not implemented"),
        // TODO: Implement Set handler
        CommandType::Set => todo!("Set handler not implemented"),
    }
}

pub fn execute(cmd: &CommandType, args: &[&str]) -> io::Result<String> {
    let handler = get_handler_for(cmd);
    handler.handle(args).map_err(Into::into)
}
