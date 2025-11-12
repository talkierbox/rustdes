use crate::commands::defs::CommandHandler;
use crate::commands::parser::{ArgumentError, ArgumentParser, ParsedArguments};

pub struct PingHandler;

impl CommandHandler for PingHandler {
    fn name(&self) -> &'static str {
        "PING"
    }

    fn parser(&self) -> ArgumentParser {
        ArgumentParser::builder(self.name())
            .optional_remainder_with_default(
                "message",
                "Custom response to send back to the client",
                ["PONG"],
            )
            .build()
    }

    fn execute(&self, args: &ParsedArguments) -> Result<String, ArgumentError> {
        Ok(args.list("message").join(" "))
    }
}
