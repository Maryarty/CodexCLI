//! The tokens of a slash command's arguments (oracle `chatwidget/slash_command_args.rs`): the
//! positional words and the named settings, shared by the commands that take both (`/permissions`,
//! `/model`, `/statusline`). Values may follow their setting or use `=`, switches never take a
//! value, a boolean setting takes `--name` and `--no-name`, and a bare `--` leaves the rest
//! positional. Quoting follows the shell (`'..'`, `".."`, a backslash). The failures are typed:
//! their words are the UI's.

/// What a command takes besides its positional words.
#[derive(Clone, Copy, Debug, Default)]
pub struct Spec<'a> {
    /// `--name value` or `--name=value`.
    pub values: &'a [&'a str],
    /// `--name`, no value.
    pub switches: &'a [&'a str],
    /// `--name` and `--no-name`, given without the dashes.
    pub booleans: &'a [&'a str],
}

/// Why the arguments did not tokenize.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArgError {
    /// A quote that does not close, or a backslash with nothing after it.
    Quoting,
    /// `Command setting {name} requires a value.`
    NeedsValue(String),
    /// `Command setting {name} does not take a value.`
    TakesNoValue(String),
    /// `Specify only one of --{name} or --no-{name}.`
    OnlyOne(String),
    /// `Command setting {name} was provided more than once.`
    Repeated(String),
    /// `Unknown command setting {name}.`
    Unknown(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Given {
    Value(String),
    Switch,
    Boolean(bool),
}

/// The words and settings of a command line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Parsed {
    pub positionals: Vec<String>,
    named: Vec<(String, Given)>,
}

impl Parsed {
    /// The value of the setting `name` (with its dashes), when it was given one.
    pub fn value(&self, name: &str) -> Option<&str> {
        self.named.iter().find_map(|(given, value)| match value {
            Given::Value(value) if given == name => Some(value.as_str()),
            _ => None,
        })
    }

    /// Whether the switch `name` (with its dashes) was given.
    pub fn has_switch(&self, name: &str) -> bool {
        self.named
            .iter()
            .any(|(given, value)| given == name && *value == Given::Switch)
    }

    /// The boolean setting `name` (without its dashes), when `--name` or `--no-name` was given.
    pub fn boolean(&self, name: &str) -> Option<bool> {
        let name = ["--", name].concat();
        self.named.iter().find_map(|(given, value)| match value {
            Given::Boolean(on) if *given == name => Some(*on),
            _ => None,
        })
    }
}

/// `shlex::split`: the words of `line`, or none when the quoting is wrong. A `#` that starts a
/// word comments out the rest of its line.
fn words(line: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        if c == '#' {
            while chars.next_if(|c| *c != '\n').is_some() {}
            continue;
        }
        let mut word = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                break;
            }
            chars.next();
            match c {
                '\\' => match chars.next()? {
                    // A backslash before a newline joins the lines.
                    '\n' => {}
                    c => word.push(c),
                },
                '\'' => loop {
                    match chars.next()? {
                        '\'' => break,
                        c => word.push(c),
                    }
                },
                '"' => loop {
                    match chars.next()? {
                        '"' => break,
                        '\\' => match chars.next()? {
                            c @ ('"' | '\\' | '$' | '`') => word.push(c),
                            '\n' => {}
                            c => {
                                word.push('\\');
                                word.push(c);
                            }
                        },
                        c => word.push(c),
                    }
                },
                c => word.push(c),
            }
        }
        words.push(word);
    }
    Some(words)
}

/// The words and settings of `line`.
pub fn parse(line: &str, spec: Spec<'_>) -> Result<Parsed, ArgError> {
    let mut words = words(line).ok_or(ArgError::Quoting)?.into_iter().peekable();
    let mut parsed = Parsed::default();
    let mut settings = true;
    while let Some(word) = words.next() {
        if settings && word == "--" {
            settings = false;
        } else if settings && word.starts_with("--") {
            let (name, given) = named(&word, &mut words, spec)?;
            if let Some((_, existing)) = parsed.named.iter().find(|(known, _)| *known == name) {
                return Err(match (existing, &given) {
                    (Given::Boolean(before), Given::Boolean(now)) if before != now => {
                        ArgError::OnlyOne(name.trim_start_matches("--").to_owned())
                    }
                    _ => ArgError::Repeated(name),
                });
            }
            parsed.named.push((name, given));
        } else {
            parsed.positionals.push(word);
        }
    }
    Ok(parsed)
}

fn named(
    word: &str,
    words: &mut std::iter::Peekable<std::vec::IntoIter<String>>,
    spec: Spec<'_>,
) -> Result<(String, Given), ArgError> {
    let (name, inline) = word
        .split_once('=')
        .map_or((word, None), |(name, value)| (name, Some(value)));
    if spec.values.contains(&name) {
        let value = inline
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .or_else(|| words.next_if(|value| !value.starts_with("--")))
            .ok_or_else(|| ArgError::NeedsValue(name.to_owned()))?;
        return Ok((name.to_owned(), Given::Value(value)));
    }
    if spec.switches.contains(&name) {
        if inline.is_some() {
            return Err(ArgError::TakesNoValue(name.to_owned()));
        }
        return Ok((name.to_owned(), Given::Switch));
    }
    for setting in spec.booleans {
        let positive = ["--", setting].concat();
        let negative = ["--no-", setting].concat();
        if name == positive || name == negative {
            if inline.is_some() {
                return Err(ArgError::TakesNoValue(name.to_owned()));
            }
            return Ok((positive, Given::Boolean(name != negative)));
        }
    }
    Err(ArgError::Unknown(name.to_owned()))
}
