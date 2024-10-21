use crate::{command, Path};

#[derive(Default)]
pub(crate) struct Parser {
    path_data: Vec<command::Data>,
    can_have_comma: bool,
    had_comma: bool,
    current_command: command::ID,
    args: [f64; 7],
    args_len: usize,
    args_capacity: usize,
    current_number: String,
    had_decminal: bool,
    cursor: usize,
}

#[derive(Debug)]
pub enum Error {
    CommandEndedTooEarly(usize),
    NoCommand,
    DuplicateComma,
    InvalidFirstCommand,
    InvalidArcSign,
    InvalidArc,
    InvalidNumber(std::num::ParseFloatError),
}

impl Parser {
    /// Returns whether the numbers of args parsed matches what's expected for the command
    fn is_flush_ready(&self) -> bool {
        self.current_command.is_implicit()
            && (self.args_len == 0 || self.args_len == self.args_capacity)
            || !self.current_command.is_implicit() && self.args_len == self.args_capacity
    }

    /// Resets `args`, `args_len`, `args_capacity`, and `can_has_comma` for the new command
    /// The old command is collated and pushed to `path_data`
    fn flush_args(&mut self, command: &command::ID) -> Result<(), Error> {
        if !self.is_flush_ready() {
            Err(Error::CommandEndedTooEarly(self.cursor))?;
        }
        self.args_capacity = command.args();
        if self.current_command.is_implicit() && self.args_len == 0 {
            self.current_command = command.clone();
            return Ok(());
        }
        let is_implicit = match self.path_data.last() {
            Some(command::Data::Implicit(c)) => c.id().next_implicit() == self.current_command,
            Some(c) => c.id().next_implicit() == self.current_command,
            _ => false,
        };
        let flushed_args: [f64; 7] = std::mem::replace(&mut self.args, [0.0; 7]);
        self.args_len = 0;
        self.can_have_comma = false;

        let from_command = if is_implicit {
            &command::ID::Implicit(Box::new(self.current_command.clone()))
        } else {
            &self.current_command
        };
        self.path_data
            .push(command::Data::from((from_command, flushed_args)));
        if !command.is_none() && self.args_capacity == 0 {
            self.path_data
                .push(command::Data::from((command, flushed_args)));
        }
        self.current_command = command.clone();
        Ok(())
    }

    fn done(&mut self) -> Path {
        Path(std::mem::take(&mut self.path_data))
    }

    fn next_command(&mut self, command: &command::ID) -> Result<(), Error> {
        if self.had_comma {
            Err(Error::DuplicateComma)?;
        }
        if self.current_command.is_none() {
            // MoveTo should be leading command
            if !matches!(command, command::ID::MoveBy | command::ID::MoveTo) {
                Err(Error::InvalidFirstCommand)?;
            }
            self.current_command = command.clone();
            self.args_capacity = self.current_command.args();
            return Ok(());
        } else if !self.is_flush_ready() {
            // stop if previous arguments are not flushed
            Err(Error::CommandEndedTooEarly(self.cursor))?;
        }
        self.flush_args(command)?;
        Ok(())
    }

    fn process_number(&mut self) -> Result<(), Error> {
        let number = std::mem::take(&mut self.current_number)
            .parse::<f64>()
            .map_err(Error::InvalidNumber)?;
        self.args[self.args_len] = number;
        self.args_len += 1;
        self.can_have_comma = true;
        self.had_comma = false;
        self.had_decminal = false;
        Ok(())
    }

    pub fn parse(&mut self, definition: impl Into<String>) -> Result<Path, Error> {
        self.cursor = 0;
        for char in definition.into().chars() {
            if char.is_whitespace() && self.current_number.is_empty() {
                continue;
            }

            // Allow comma only between arguments
            if char == ',' && self.current_number.is_empty() {
                if self.had_comma {
                    Err(Error::DuplicateComma)?;
                }
                self.had_comma = true;
                continue;
            }
            self.had_comma = false;
            if let Ok(command_id) = command::ID::try_from(char) {
                if !self.current_number.is_empty() {
                    self.process_number()?;
                }
                self.next_command(&command_id)?;
                continue;
            }

            // avoid parsing arguments if no command is detected
            if self.current_command.is_none() {
                Err(Error::NoCommand)?;
            }
            if (!char.is_numeric() && !matches!(char, '+' | '-' | '.' | 'e' | 'E'))
                // '.' is start of new number
                || (self.had_decminal && char == '.' && !self.current_number.ends_with('e') && !self.current_number.ends_with('-'))
                // '-' is start of new number
                || (!self.current_number.is_empty()
                    && !self.current_number.ends_with('e')
                    && char == '-')
            {
                self.process_number()?;
                self.had_comma = char == ',';
                if char == '.' {
                    self.current_number.push(char);
                    self.had_decminal = true;
                } else if char == '-' {
                    self.current_number.push(char);
                }
                if self.args_len != self.args_capacity {
                    continue;
                }
                // flush arguments when capacity is reached
                self.flush_args(&command::ID::Implicit(match self.current_command {
                    command::ID::MoveTo => Box::new(command::ID::LineTo),
                    command::ID::MoveBy => Box::new(command::ID::LineBy),
                    _ => Box::new(self.current_command.clone()),
                }))?;
                continue;
            }
            // read next argument
            if matches!(
                self.current_command,
                command::ID::ArcTo | command::ID::ArcBy
            ) {
                let number = match char {
                    // don't allow sign on first two args
                    '+' | '-' if self.args_len <= 1 => {
                        return Err(Error::InvalidArcSign)?;
                    }
                    '0' if (3..=4).contains(&self.args_len) => 0.0,
                    '1' if (3..=4).contains(&self.args_len) => 1.0,
                    '+' | '-' | '.' => {
                        self.current_number.push(char);
                        continue;
                    }
                    char if char.is_numeric() => {
                        self.current_number.push(char);
                        continue;
                    }
                    _ => {
                        return Err(Error::InvalidArc)?;
                    }
                };
                self.args[self.args_len] = number;
                self.args_len += 1;
            } else {
                self.had_decminal = self.had_decminal || char == '.';
                self.current_number.push(char);
            }
            self.cursor += 1;
        }
        if !self.current_number.is_empty() {
            self.process_number()?;
        }
        self.flush_args(&command::ID::None)?;
        Ok(self.done())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fmt = match self {
            Self::CommandEndedTooEarly(_) => "A path command ended too early",
            Self::NoCommand => "Expected a path command",
            Self::DuplicateComma => "Found unexpected comma in path command",
            Self::InvalidFirstCommand => "Expected path to start with `m` or `M`",
            Self::InvalidArcSign => "Unexpected sign given on one of first two `a` or `A` commands",
            Self::InvalidArc => "Badly formatted `a` or `A` command",
            Self::InvalidNumber(e) => &format!("Failed to parse number in path: {e}"),
        };
        f.write_str(fmt)?;
        Ok(())
    }
}

impl std::error::Error for Error {}
