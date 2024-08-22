use std::fmt::Write;

#[derive(Debug)]
pub enum DataCommand {
    /// M
    /// Move the current point to coordinate `x`, `y`. Any subsequent coordinate pair(s) are
    /// interpreted as parameter(s) for implicit absolute `LineTo` (L) command(s)
    MoveTo([f64; 2]),
    /// m
    MoveBy([f64; 2]),
    /// Z or z
    ClosePath,
    /// L
    LineTo([f64; 2]),
    /// l
    LineBy([f64; 2]),
    /// H
    HorizontalLineTo([f64; 1]),
    /// h
    HorizontalLineBy([f64; 1]),
    /// V
    VerticalLineTo([f64; 1]),
    /// v
    VerticalLineBy([f64; 1]),
    /// C
    CubicBezierTo([f64; 6]),
    /// c
    CubicBezierBy([f64; 6]),
    /// S
    SmoothBezierTo([f64; 4]),
    /// s
    SmoothBezierBy([f64; 4]),
    /// Q
    QuadraticBezierTo([f64; 4]),
    /// q
    QuadraticBezierBy([f64; 4]),
    /// T
    SmoothQuadraticBezierTo([f64; 2]),
    /// t
    SmoothQuadraticBezierBy([f64; 2]),
    /// A
    ArcTo([f64; 7]),
    /// a
    ArcBy([f64; 7]),
    Implicit(Box<DataCommand>),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum CommandId {
    /// M
    /// Move the current point to coordinate `x`, `y`. Any subsequent coordinate pair(s) are
    /// interpreted as parameter(s) for implicit absolute `LineTo` (L) command(s)
    MoveTo,
    /// m
    MoveBy,
    /// Z or z
    ClosePath,
    /// L
    LineTo,
    /// l
    LineBy,
    /// H
    HorizontalLineTo,
    /// h
    HorizontalLineBy,
    /// V
    VerticalLineTo,
    /// v
    VerticalLineBy,
    /// C
    CubicBezierTo,
    /// c
    CubicBezierBy,
    /// S
    SmoothBezierTo,
    /// s
    SmoothBezierBy,
    /// Q
    QuadraticBezierTo,
    /// q
    QuadraticBezierBy,
    /// T
    SmoothQuadraticBezierTo,
    /// t
    SmoothQuadraticBezierBy,
    /// A
    ArcTo,
    /// a
    ArcBy,
    /// The absence of any command
    #[default]
    None,
    Implicit(Box<CommandId>),
}

#[derive(Debug)]
pub struct Path(Vec<DataCommand>);

#[derive(Default)]
struct Parser {
    path_data: Vec<DataCommand>,
    can_have_comma: bool,
    had_comma: bool,
    current_command: CommandId,
    args: [f64; 7],
    args_len: usize,
    args_capacity: usize,
    current_number: String,
    had_decminal: bool,
}

impl Path {
    pub fn parse(definition: &impl ToString) -> Result<Self, Self> {
        Parser::default().parse(definition)
    }
}

impl std::fmt::Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.len() == 1 {
            self.0.first().unwrap().fmt(f)?;
            return Ok(());
        }
        self.0
            .windows(2)
            .enumerate()
            .try_for_each(|(i, window)| -> std::fmt::Result {
                let prev = &window[0];
                let current = &window[1];
                if i == 0 {
                    prev.fmt(f)?;
                }
                #[cfg(test)]
                dbg!(format!("{prev}~~~{current}"));
                if current.is_space_needed(prev) {
                    dbg!("path::Path::fmt: adding space between implicit command");
                    f.write_char(' ')?;
                }
                current.fmt(f)?;
                Ok(())
            })
    }
}

impl Parser {
    fn is_flush_ready(&self) -> bool {
        self.current_command.is_implicit()
            && (self.args_len == 0 || self.args_len == self.args_capacity)
            || !self.current_command.is_implicit() && self.args_len == self.args_capacity
    }

    /// Resets `args`, `args_len`, `args_capacity`, and `can_has_comma` for the new command
    /// The old command is collated and pushed to `path_data`
    fn flush_args(&mut self, command: &CommandId) -> Result<(), Path> {
        if !self.is_flush_ready() {
            dbg!("path::Parser::flush_args: failed, flushed too early");
            Err(self.done())?;
        }
        self.args_capacity = command.args();
        if self.current_command.is_implicit() && self.args_len == 0 {
            self.current_command = command.clone();
            return Ok(());
        }
        let is_implicit = match self.path_data.last() {
            Some(DataCommand::Implicit(c)) => c.id() == self.current_command,
            Some(c) => c.id() == self.current_command,
            _ => false,
        };
        let flushed_args: [f64; 7] = std::mem::replace(&mut self.args, [0.0; 7]);
        self.args_len = 0;
        self.can_have_comma = false;

        let from_command = if is_implicit {
            &CommandId::Implicit(Box::new(self.current_command.clone()))
        } else {
            &self.current_command
        };
        dbg!(
            "path::Parser::flush_args: pushing command to path_data",
            &from_command
        );
        self.path_data
            .push(DataCommand::from((from_command, flushed_args)));
        if !command.is_none() && self.args_capacity == 0 {
            self.path_data
                .push(DataCommand::from((command, flushed_args)));
        }
        self.current_command = command.clone();
        Ok(())
    }

    fn done(&mut self) -> Path {
        Path(std::mem::take(&mut self.path_data))
    }

    fn next_command(&mut self, command: &CommandId) -> Result<(), Path> {
        if self.had_comma {
            dbg!("path::Parser::next_command: failed due to prior comma");
            Err(self.done())?;
        }
        if self.current_command.is_none() {
            // MoveTo should be leading command
            if !matches!(command, CommandId::MoveBy | CommandId::MoveTo) {
                dbg!(
                    "path::Parser::next_command: failed, first command isn't m/M",
                    command
                );
                Err(self.done())?;
            }
            self.current_command = command.clone();
            self.args_capacity = self.current_command.args();
            return Ok(());
        } else if !self.is_flush_ready() {
            // stop if previous arguments are not flushed
            dbg!(
                "path::Parser::next_command: failed, command ended too early",
                self.args_len,
                self.args_capacity
            );
            Err(self.done())?;
        }
        self.flush_args(command)?;
        Ok(())
    }

    fn process_number(&mut self) -> Result<(), Path> {
        let number = std::mem::take(&mut self.current_number)
            .parse::<f64>()
            .map_err(|error| {
                dbg!("path::Parser::process_number: failed", error);
                self.done()
            })?;
        self.args[self.args_len] = number;
        self.args_len += 1;
        self.can_have_comma = true;
        self.had_comma = false;
        self.had_decminal = false;
        Ok(())
    }

    fn parse(&mut self, definition: &impl ToString) -> Result<Path, Path> {
        dbg!("path::Parser::parse: starting");
        for char in definition.to_string().chars() {
            dbg!("path::Parser::parse: parsing char", char);
            if char.is_whitespace() && self.current_number.is_empty() {
                dbg!("path::Parser::parse: char is whitespace");
                continue;
            }

            // Allow comma only between arguments
            if char == ',' && self.current_number.is_empty() {
                if self.had_comma {
                    dbg!("path::Parser::parse: failed due to multiple commas");
                    Err(self.done())?;
                }
                dbg!("path::Parser::parse: char is valid comma");
                self.had_comma = true;
                continue;
            }
            if let Ok(command_id) = CommandId::try_from(char) {
                dbg!("path::Parser::parse: char is command");
                if !self.current_number.is_empty() {
                    self.process_number()?;
                }
                self.next_command(&command_id)?;
                continue;
            }

            // avoid parsing arguments if no command is detected
            if self.current_command.is_none() {
                dbg!("path::Parser::parse: failed as no command is provided");
                Err(self.done())?;
            }
            if (!char.is_numeric() && !matches!(char, '+' | '-' | '.' | 'e' | 'E'))
                // '.' is start of new number
                || (self.had_decminal && char == '.')
                // '-' is start of new number
                || (!self.current_number.is_empty()
                    && !self.current_number.ends_with('e')
                    && char == '-')
            {
                dbg!("path::Parser::parse: char is after number");
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
                dbg!("path::Parser::parse: char is last of args");
                self.flush_args(&CommandId::Implicit(match self.current_command {
                    CommandId::MoveTo => Box::new(CommandId::LineTo),
                    CommandId::MoveBy => Box::new(CommandId::LineBy),
                    _ => Box::new(self.current_command.clone()),
                }))?;
                continue;
            }
            // read next argument
            if matches!(self.current_command, CommandId::ArcTo | CommandId::ArcBy) {
                dbg!("path::Parser::parse: char is arc arg");
                let number = match char {
                    // don't allow sign on first two args
                    '+' | '-' if self.args_len <= 1 => {
                        dbg!("path::Parser::parse: failed because sign was given within first two args of A/a");
                        return Err(self.done())?;
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
                        dbg!("path::Parser::parse: failed due to unexpected char");
                        return Err(self.done())?;
                    }
                };
                self.args[self.args_len] = number;
                self.args_len += 1;
            } else {
                self.had_decminal = self.had_decminal || char == '.';
                self.current_number.push(char);
                dbg!("path::Parser::parse: char is arg", &self.current_number);
            }
        }
        if !self.current_number.is_empty() {
            self.process_number()?;
        }
        dbg!(
            "path::Parser::parse: reached end",
            &self.current_number,
            self.args_len,
            self.args_capacity
        );
        self.flush_args(&CommandId::None)?;
        Ok(self.done())
    }
}

impl DataCommand {
    fn id(&self) -> CommandId {
        match self {
            Self::MoveTo(..) => CommandId::MoveTo,
            Self::MoveBy(..) => CommandId::MoveBy,
            Self::ClosePath => CommandId::ClosePath,
            Self::LineTo(..) => CommandId::LineTo,
            Self::LineBy(..) => CommandId::LineBy,
            Self::HorizontalLineTo(..) => CommandId::HorizontalLineTo,
            Self::HorizontalLineBy(..) => CommandId::HorizontalLineBy,
            Self::VerticalLineTo(..) => CommandId::VerticalLineTo,
            Self::VerticalLineBy(..) => CommandId::VerticalLineBy,
            Self::CubicBezierTo(..) => CommandId::CubicBezierTo,
            Self::CubicBezierBy(..) => CommandId::CubicBezierBy,
            Self::SmoothBezierTo(..) => CommandId::SmoothBezierTo,
            Self::SmoothBezierBy(..) => CommandId::SmoothBezierBy,
            Self::QuadraticBezierTo(..) => CommandId::QuadraticBezierTo,
            Self::QuadraticBezierBy(..) => CommandId::QuadraticBezierBy,
            Self::SmoothQuadraticBezierTo(..) => CommandId::SmoothQuadraticBezierTo,
            Self::SmoothQuadraticBezierBy(..) => CommandId::SmoothQuadraticBezierBy,
            Self::ArcTo(..) => CommandId::ArcTo,
            Self::ArcBy(..) => CommandId::ArcBy,
            Self::Implicit(command) => CommandId::Implicit(Box::new(command.id())),
        }
    }

    fn args(&self) -> &[f64] {
        match self {
            Self::MoveTo(a)
            | Self::MoveBy(a)
            | Self::LineTo(a)
            | Self::LineBy(a)
            | Self::SmoothQuadraticBezierTo(a)
            | Self::SmoothQuadraticBezierBy(a) => a,
            Self::ClosePath => &[],
            Self::HorizontalLineTo(a)
            | Self::HorizontalLineBy(a)
            | Self::VerticalLineTo(a)
            | Self::VerticalLineBy(a) => a,
            Self::SmoothBezierTo(a)
            | Self::SmoothBezierBy(a)
            | Self::QuadraticBezierTo(a)
            | Self::QuadraticBezierBy(a) => a,
            Self::CubicBezierTo(a) | Self::CubicBezierBy(a) => a,
            Self::ArcTo(a) | Self::ArcBy(a) => a,
            Self::Implicit(a) => a.args(),
        }
    }

    fn is_implicit(&self) -> bool {
        matches!(self, Self::Implicit(_))
    }

    fn is_space_needed(&self, prev: &Self) -> bool {
        self.is_implicit() && prev.args().last().is_some_and(|n| (n % 1.0) == 0.0)
    }
}

impl From<(&CommandId, [f64; 7])> for DataCommand {
    fn from(value: (&CommandId, [f64; 7])) -> Self {
        let (command_id, args) = value;
        match command_id {
            CommandId::MoveTo => Self::MoveTo([args[0], args[1]]),
            CommandId::MoveBy => Self::MoveBy([args[0], args[1]]),
            CommandId::ClosePath => Self::ClosePath,
            CommandId::LineTo => Self::LineTo([args[0], args[1]]),
            CommandId::LineBy => Self::LineBy([args[0], args[1]]),
            CommandId::HorizontalLineTo => Self::HorizontalLineTo([args[0]]),
            CommandId::HorizontalLineBy => Self::HorizontalLineBy([args[0]]),
            CommandId::VerticalLineTo => Self::VerticalLineTo([args[0]]),
            CommandId::VerticalLineBy => Self::VerticalLineBy([args[0]]),
            CommandId::CubicBezierTo => {
                Self::CubicBezierTo([args[0], args[1], args[2], args[3], args[4], args[5]])
            }
            CommandId::CubicBezierBy => {
                Self::CubicBezierBy([args[0], args[1], args[2], args[3], args[4], args[5]])
            }
            CommandId::SmoothBezierTo => Self::SmoothBezierTo([args[0], args[1], args[2], args[3]]),
            CommandId::SmoothBezierBy => Self::SmoothBezierBy([args[0], args[1], args[2], args[3]]),
            CommandId::QuadraticBezierTo => {
                Self::QuadraticBezierTo([args[0], args[1], args[2], args[3]])
            }
            CommandId::QuadraticBezierBy => {
                Self::QuadraticBezierBy([args[0], args[1], args[2], args[3]])
            }
            CommandId::SmoothQuadraticBezierTo => Self::SmoothQuadraticBezierTo([args[0], args[1]]),
            CommandId::SmoothQuadraticBezierBy => Self::SmoothQuadraticBezierBy([args[0], args[1]]),
            CommandId::ArcTo => Self::ArcTo([
                args[0], args[1], args[2], args[3], args[4], args[5], args[6],
            ]),
            CommandId::ArcBy => Self::ArcBy([
                args[0], args[1], args[2], args[3], args[4], args[5], args[6],
            ]),
            CommandId::None => unreachable!(),
            CommandId::Implicit(command) => {
                DataCommand::Implicit(Box::new(DataCommand::from((command.as_ref(), args))))
            }
        }
    }
}

impl std::fmt::Display for DataCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.id().fmt(f)?;
        if self.args().len() == 1 {
            self.args().first().unwrap().fmt(f)?;
            return Ok(());
        }
        self.args()
            .windows(2)
            .enumerate()
            .try_for_each(|(i, window)| -> std::fmt::Result {
                let previous = &window[0];
                let current = &window[1];
                let to_short_string = |n: &f64| -> String {
                    let mut s = ryu::Buffer::new().format(*n).to_owned();
                    if s == "0.0" || s == "-0.0" {
                        return String::from("0");
                    }
                    // Remove leading zero
                    if s.starts_with("0.") {
                        s.remove(0);
                    } else if s.starts_with("-0.") {
                        s.remove(1);
                    }
                    if s.ends_with(".0") {
                        s.pop();
                        s.pop();
                    }
                    s
                };
                if i == 0 {
                    to_short_string(previous).fmt(f)?;
                }
                let s = to_short_string(current);
                if current >= &1.0
                    || (previous % 1.0 == 0.0 && s.chars().next().is_some_and(char::is_numeric))
                {
                    f.write_char(' ')?;
                }
                s.fmt(f)?;
                Ok(())
            })?;
        Ok(())
    }
}

impl CommandId {
    fn args(&self) -> usize {
        match self {
            Self::ClosePath | Self::None => 0,
            Self::HorizontalLineTo
            | Self::HorizontalLineBy
            | Self::VerticalLineTo
            | Self::VerticalLineBy => 1,
            Self::LineTo
            | Self::LineBy
            | Self::MoveTo
            | Self::MoveBy
            | Self::SmoothQuadraticBezierTo
            | Self::SmoothQuadraticBezierBy => 2,
            Self::SmoothBezierTo
            | Self::SmoothBezierBy
            | Self::QuadraticBezierTo
            | Self::QuadraticBezierBy => 4,
            Self::CubicBezierTo | Self::CubicBezierBy => 6,
            Self::ArcTo | Self::ArcBy => 7,
            Self::Implicit(command) => command.args(),
        }
    }

    fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    fn is_implicit(&self) -> bool {
        matches!(self, Self::Implicit(_))
    }
}

impl TryFrom<char> for CommandId {
    type Error = ();

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            'M' => Ok(Self::MoveTo),
            'm' => Ok(Self::MoveBy),
            'L' => Ok(Self::LineTo),
            'l' => Ok(Self::LineBy),
            'H' => Ok(Self::HorizontalLineTo),
            'h' => Ok(Self::HorizontalLineBy),
            'V' => Ok(Self::VerticalLineTo),
            'v' => Ok(Self::VerticalLineBy),
            'C' => Ok(Self::CubicBezierTo),
            'c' => Ok(Self::CubicBezierBy),
            'S' => Ok(Self::SmoothBezierTo),
            's' => Ok(Self::SmoothBezierBy),
            'Q' => Ok(Self::QuadraticBezierTo),
            'q' => Ok(Self::QuadraticBezierBy),
            'T' => Ok(Self::SmoothQuadraticBezierTo),
            't' => Ok(Self::SmoothQuadraticBezierBy),
            'A' => Ok(Self::ArcTo),
            'a' => Ok(Self::ArcBy),
            'Z' | 'z' => Ok(Self::ClosePath),
            _ => Err(()),
        }
    }
}

impl From<&CommandId> for char {
    fn from(value: &CommandId) -> Self {
        match value {
            CommandId::MoveTo => 'M',
            CommandId::MoveBy => 'm',
            CommandId::ClosePath => 'Z',
            CommandId::LineTo => 'L',
            CommandId::LineBy => 'l',
            CommandId::HorizontalLineTo => 'H',
            CommandId::HorizontalLineBy => 'h',
            CommandId::VerticalLineTo => 'V',
            CommandId::VerticalLineBy => 'v',
            CommandId::CubicBezierTo => 'C',
            CommandId::CubicBezierBy => 'c',
            CommandId::SmoothBezierTo => 'S',
            CommandId::SmoothBezierBy => 's',
            CommandId::QuadraticBezierTo => 'Q',
            CommandId::QuadraticBezierBy => 'q',
            CommandId::SmoothQuadraticBezierTo => 'T',
            CommandId::SmoothQuadraticBezierBy => 't',
            CommandId::ArcTo => 'A',
            CommandId::ArcBy => 'a',
            CommandId::None => unreachable!(),
            CommandId::Implicit(_) => ' ',
        }
    }
}

impl From<CommandId> for char {
    fn from(value: CommandId) -> Self {
        (&value).into()
    }
}

impl std::fmt::Display for CommandId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_implicit() {
            return Ok(());
        }
        f.write_char(self.into())?;
        Ok(())
    }
}

#[test]
fn test_path_parse() {
    // Should parse single command
    insta::assert_snapshot!(dbg!(Path::parse(&"M 10,50").unwrap()));

    // Should parse multiple commands
    insta::assert_snapshot!(dbg!(Path::parse(
        &"M 10,50 C 20,30 40,50 60,70 C 10,20 30,40 50,60"
    )
    .unwrap()));

    // Should parse arc
    insta::assert_snapshot!(dbg!(Path::parse(&"m-0,1a 25,25 -30 0,1 0,0").unwrap()));

    // Should parse implicit
    insta::assert_snapshot!(dbg!(Path::parse(
        &"M 10,50 C 1,2 3,4 5,6.5 .1 .2 .3 .4 .5 -.05176e-005"
    )
    .unwrap()));

    // Should parse minified
    insta::assert_snapshot!(dbg!(Path::parse(
        &"M10 50C1 2 3 4 5 6.5.1.2.3.4.5-5.176e-7"
    )
    .unwrap()));

    // Should error when command isn't given
    assert!(dbg!(Path::parse(&"0,0")).is_err());

    // Should error when args are missing
    assert!(dbg!(Path::parse(&"m1")).is_err());
}
