use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::fmt;
use std::io;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgumentArity {
    Single,
    Remainder,
}

#[derive(Clone, Debug)]
pub struct ArgumentDefinition {
    pub name: &'static str,
    pub description: &'static str,
    pub required: bool,
    pub arity: ArgumentArity,
    pub default: Option<Vec<String>>,
}

impl ArgumentDefinition {
    pub fn required(name: &'static str, description: &'static str) -> Self {
        Self {
            name,
            description,
            required: true,
            arity: ArgumentArity::Single,
            default: None,
        }
    }

    pub fn optional(name: &'static str, description: &'static str) -> Self {
        Self {
            name,
            description,
            required: false,
            arity: ArgumentArity::Single,
            default: None,
        }
    }

    pub fn optional_with_default(
        name: &'static str,
        description: &'static str,
        default: impl Into<String>,
    ) -> Self {
        Self {
            name,
            description,
            required: false,
            arity: ArgumentArity::Single,
            default: Some(vec![default.into()]),
        }
    }

    pub fn required_remainder(name: &'static str, description: &'static str) -> Self {
        Self {
            name,
            description,
            required: true,
            arity: ArgumentArity::Remainder,
            default: None,
        }
    }

    pub fn optional_remainder(name: &'static str, description: &'static str) -> Self {
        Self {
            name,
            description,
            required: false,
            arity: ArgumentArity::Remainder,
            default: None,
        }
    }

    pub fn optional_remainder_with_default<I, S>(
        name: &'static str,
        description: &'static str,
        default: I,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            name,
            description,
            required: false,
            arity: ArgumentArity::Remainder,
            default: Some(default.into_iter().map(Into::into).collect()),
        }
    }

    pub fn usage_token(&self) -> String {
        let base = match self.arity {
            ArgumentArity::Single => self.name.to_string(),
            ArgumentArity::Remainder => format!("{}...", self.name),
        };

        let token = if self.required {
            format!("<{}>", base)
        } else {
            format!("[{}]", base)
        };

        if matches!(self.arity, ArgumentArity::Single) {
            if let Some(default) = &self.default {
                if let Some(value) = default.first() {
                    return format!("{}={}", token, value);
                }
            }
        }

        token
    }

    pub fn summary(&self) -> String {
        let requirement = if self.required {
            "required"
        } else {
            "optional"
        };
        let arity = match self.arity {
            ArgumentArity::Single => "single",
            ArgumentArity::Remainder => "variadic",
        };

        let mut summary = format!(
            "{} ({} {}): {}",
            self.name, requirement, arity, self.description
        );

        if let Some(default) = &self.default {
            if !default.is_empty() {
                let rendered = if matches!(self.arity, ArgumentArity::Single) {
                    default.first().cloned().unwrap_or_default()
                } else {
                    default.join(" ")
                };

                summary.push_str(&format!(" [default: {}]", rendered));
            }
        }

        summary
    }
}

#[derive(Clone, Debug)]
pub struct ArgumentParser {
    command_name: &'static str,
    specs: Vec<ArgumentDefinition>,
}

impl ArgumentParser {
    pub fn new(command_name: &'static str, specs: Vec<ArgumentDefinition>) -> Self {
        Self {
            command_name,
            specs,
        }
    }

    pub fn builder(command_name: &'static str) -> ArgumentParserBuilder {
        ArgumentParserBuilder::new(command_name)
    }

    pub fn usage(&self) -> String {
        let tokens: Vec<String> = self
            .specs
            .iter()
            .map(ArgumentDefinition::usage_token)
            .collect();
        if tokens.is_empty() {
            format!("Usage: {}", self.command_name)
        } else {
            format!("Usage: {} {}", self.command_name, tokens.join(" "))
        }
    }

    pub fn usage_with_details(&self) -> String {
        let mut sections = vec![self.usage()];

        if !self.specs.is_empty() {
            let mut details = vec!["Arguments:".to_string()];
            for spec in &self.specs {
                details.push(format!("  {}", spec.summary()));
            }
            sections.push(details.join("\n"));
        }

        sections.join("\n")
    }

    fn error(&self, message: impl Into<String>) -> ArgumentError {
        ArgumentError::new(self.command_name, message.into(), self.usage_with_details())
    }

    pub fn parse(&self, args: &[&str]) -> Result<ParsedArguments, ArgumentError> {
        let raw: Vec<String> = args
            .iter()
            .map(|arg| arg.trim())
            .filter(|arg| !arg.is_empty())
            .map(|arg| arg.to_string())
            .collect();

        let mut queue: VecDeque<String> = VecDeque::from(raw.clone());
        let mut values: HashMap<&'static str, Vec<String>> = HashMap::new();
        let mut missing: Vec<&'static str> = Vec::new();

        for (index, spec) in self.specs.iter().enumerate() {
            match spec.arity {
                ArgumentArity::Single => {
                    if let Some(value) = queue.pop_front() {
                        values.insert(spec.name, vec![value]);
                    } else if let Some(default) = spec.default.clone() {
                        values.insert(spec.name, default);
                    } else {
                        if spec.required {
                            missing.push(spec.name);
                        }
                        values.insert(spec.name, Vec::new());
                    }
                }
                ArgumentArity::Remainder => {
                    if index != self.specs.len() - 1 {
                        return Err(self.error(format!(
                            "Argument '{}' captures the remaining input and must be the final argument",
                            spec.name
                        )));
                    }

                    let remainder: Vec<String> = queue.drain(..).collect();
                    if remainder.is_empty() {
                        if let Some(default) = spec.default.clone() {
                            values.insert(spec.name, default);
                        } else {
                            if spec.required {
                                missing.push(spec.name);
                            }
                            values.insert(spec.name, Vec::new());
                        }
                    } else {
                        values.insert(spec.name, remainder);
                    }
                }
            }
        }

        if !queue.is_empty() {
            let extras = queue.into_iter().collect::<Vec<_>>().join(" ");
            return Err(self.error(format!("Unexpected argument(s) starting at '{}'", extras)));
        }

        if !missing.is_empty() {
            let formatted = missing.join(", ");
            return Err(self.error(format!("Missing required argument(s): {}", formatted)));
        }

        Ok(ParsedArguments {
            command_name: self.command_name,
            raw,
            order: self.specs.iter().map(|spec| spec.name).collect(),
            values,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ArgumentParserBuilder {
    command_name: &'static str,
    specs: Vec<ArgumentDefinition>,
}

impl ArgumentParserBuilder {
    fn new(command_name: &'static str) -> Self {
        Self {
            command_name,
            specs: Vec::new(),
        }
    }

    pub fn arg(mut self, definition: ArgumentDefinition) -> Self {
        self.specs.push(definition);
        self
    }

    pub fn required(self, name: &'static str, description: &'static str) -> Self {
        self.arg(ArgumentDefinition::required(name, description))
    }

    pub fn optional(self, name: &'static str, description: &'static str) -> Self {
        self.arg(ArgumentDefinition::optional(name, description))
    }

    pub fn optional_with_default(
        self,
        name: &'static str,
        description: &'static str,
        default: impl Into<String>,
    ) -> Self {
        self.arg(ArgumentDefinition::optional_with_default(
            name,
            description,
            default,
        ))
    }

    pub fn required_remainder(self, name: &'static str, description: &'static str) -> Self {
        self.arg(ArgumentDefinition::required_remainder(name, description))
    }

    pub fn optional_remainder(self, name: &'static str, description: &'static str) -> Self {
        self.arg(ArgumentDefinition::optional_remainder(name, description))
    }

    pub fn optional_remainder_with_default<I, S>(
        self,
        name: &'static str,
        description: &'static str,
        default: I,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.arg(ArgumentDefinition::optional_remainder_with_default(
            name,
            description,
            default,
        ))
    }

    pub fn build(self) -> ArgumentParser {
        ArgumentParser::new(self.command_name, self.specs)
    }
}

#[derive(Debug)]
pub struct ParsedArguments {
    command_name: &'static str,
    raw: Vec<String>,
    order: Vec<&'static str>,
    values: HashMap<&'static str, Vec<String>>,
}

impl ParsedArguments {
    pub fn command_name(&self) -> &str {
        self.command_name
    }

    pub fn raw(&self) -> &[String] {
        &self.raw
    }

    pub fn names(&self) -> &[&'static str] {
        &self.order
    }

    pub fn has(&self, name: &str) -> bool {
        self.values
            .get(name)
            .map(|list| !list.is_empty())
            .unwrap_or(false)
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.values
            .get(name)
            .and_then(|list| list.first())
            .map(|value| value.as_str())
    }

    pub fn get_or<'a>(&'a self, name: &str, fallback: &'a str) -> &'a str {
        self.get(name).unwrap_or(fallback)
    }

    pub fn get_all(&self, name: &str) -> Option<&[String]> {
        self.values.get(name).map(|list| list.as_slice())
    }

    pub fn get_joined(&self, name: &str, separator: &str) -> Option<String> {
        self.values.get(name).map(|list| list.join(separator))
    }

    pub fn list(&self, name: &str) -> &[String] {
        const EMPTY: &[String] = &[];
        self.values
            .get(name)
            .map(|list| list.as_slice())
            .unwrap_or(EMPTY)
    }

    pub fn pretty(&self) -> String {
        if self.order.is_empty() {
            return format!("{}: <no arguments>", self.command_name);
        }

        let mut lines = vec![format!("{} arguments:", self.command_name)];
        for name in &self.order {
            let rendered = match self.values.get(name) {
                Some(values) if !values.is_empty() => values.join(" "),
                _ => "<none>".to_string(),
            };
            lines.push(format!("  {}: {}", name, rendered));
        }

        lines.join("\n")
    }
}

impl fmt::Display for ParsedArguments {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.pretty())
    }
}

#[derive(Debug)]
pub struct ArgumentError {
    command_name: &'static str,
    message: String,
    usage: String,
}

impl ArgumentError {
    fn new(command_name: &'static str, message: String, usage: String) -> Self {
        Self {
            command_name,
            message,
            usage,
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn usage(&self) -> &str {
        &self.usage
    }

    pub fn command(&self) -> &str {
        self.command_name
    }

    pub fn pretty(&self) -> String {
        format!("{}\n{}", self.message, self.usage)
    }
}

impl fmt::Display for ArgumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.pretty())
    }
}

impl Error for ArgumentError {}

impl From<ArgumentError> for io::Error {
    fn from(err: ArgumentError) -> Self {
        io::Error::new(io::ErrorKind::InvalidInput, err.to_string())
    }
}