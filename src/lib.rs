#![feature(specialization)]

//! Simply call `prompt`, `prompt_opt`, or `prompt_default` on a `Promptable` type:
//!
//! ```no_run
//! # use std::path::PathBuf;
//! use promptly::Promptable;
//!
//! // Prompt until a non-empty string is provided
//! let name = String::prompt("Enter your name");
//!
//! // Prompt for other `FromStr` types
//! let age = u32::prompt("Enter your age");
//!
//! // Prompt for optional paths with path completion. Returns `None` if empty input.
//! let photo = PathBuf::prompt_opt("Enter a path to a profile picture");
//!
//! // Prompt Y/n with a default value when input is empty
//! let fallback = bool::prompt_default("Would you like to receive marketing emails", true);
//! ```

extern crate rustyline;

use rustyline::completion::{Completer, FilenameCompleter};
use rustyline::{error::ReadlineError, Editor};
use std::env;
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;

/// A trait for convenient, opinionated prompting
pub trait Promptable: Sized {
    /// Prompts for a value. Re-prompts on invalid and empty input.
    fn prompt<S: AsRef<str>>(msg: S) -> Self;

    /// Prompts for a value, returning `None` for empty input. Re-prompts on invalid input.
    fn prompt_opt<S: AsRef<str>>(msg: S) -> Option<Self>;

    /// Prompts for a value with a default value for empty input. Re-prompts on invalid input.
    ///
    /// The default value will be mentioned in the prompt message
    fn prompt_default<S: AsRef<str>>(msg: S, default: Self) -> Self;
}

/// Quick hack around specialization issues
pub trait Parseable: FromStr + Display {}
default impl<T> Parseable for T
where
    T: FromStr + Display,
{
}

/*default impl<T> Promptable for T
where
    T: FromStr + Display,
    <T as FromStr>::Err: ::std::error::Error,*/

/// Blanket impl for `FromStr` types. Re-prompts until `FromStr` parsing succeeds.
default impl<T> Promptable for T
where
    T: Parseable,
    <T as FromStr>::Err: ::std::error::Error,
{
    /// Prompt until the input parses into the specified type
    ///
    /// ```no_run
    /// use promptly::Promptable;
    /// u32::prompt("Enter your age");
    /// ```
    fn prompt<S: AsRef<str>>(msg: S) -> Self {
        prompt_parse(msg)
    }

    /// Prompt for an optional, parseable value.
    ///
    /// Returns `None` if empty, otherwise prompts until input parses into specified type.
    ///
    /// ```no_run
    /// # use std::net::IpAddr;
    /// use promptly::Promptable;
    /// IpAddr::prompt_opt("Enter your IP Address (optional)");
    /// ```
    fn prompt_opt<S: AsRef<str>>(msg: S) -> Option<Self> {
        prompt_parse_opt(msg)
    }

    /// Prompt for a parseable value with a provided fallback value if empty.
    ///
    /// ```no_run
    /// use promptly::Promptable;
    /// u32::prompt_default("Enter the year", 2018);
    /// ```
    ///
    /// Default value is visible in the prompt as: `(default=USA)`
    fn prompt_default<S: AsRef<str>>(msg: S, default: Self) -> Self {
        let msg = format!("{} (default={})", msg.as_ref(), default);
        prompt_parse_opt(msg).unwrap_or(default)
    }
}

impl Promptable for String {
    /// Prompt until you get a non-empty string
    ///
    /// ```no_run
    /// use promptly::Promptable;
    /// String::prompt("Enter your name");
    /// ```
    fn prompt<S: AsRef<str>>(msg: S) -> Self {
        Prompter::new().prompt_nonempty(msg)
    }

    /// Prompt for an optional string
    ///
    /// ```no_run
    /// use promptly::Promptable;
    /// String::prompt_opt("Enter your phone number (optional)");
    /// ```
    fn prompt_opt<S: AsRef<str>>(msg: S) -> Option<Self> {
        Prompter::new().prompt_opt(msg)
    }

    /// Prompt for a string with a provided fallback value if empty.
    ///
    /// ```no_run
    /// use promptly::Promptable;
    /// String::prompt_default("Enter your country", "USA".into());
    /// ```
    ///
    /// Default value is visible in the prompt as: `(default=USA)`
    fn prompt_default<S: AsRef<str>>(msg: S, default: Self) -> Self {
        let msg = format!("{} (default={})", msg.as_ref(), default);
        Prompter::new().prompt_opt(msg).unwrap_or(default)
    }
}

/// PathBuf prompting will use a path autocompleter
impl Promptable for PathBuf {
    /// Prompt until you get a non-empty path
    fn prompt<S: AsRef<str>>(msg: S) -> Self {
        prompt_path(msg)
    }
    /// Prompt for an optional path
    fn prompt_opt<S: AsRef<str>>(msg: S) -> Option<Self> {
        prompt_path_opt(msg)
    }
    /// Prompt for a path with a provided fallback value if empty
    fn prompt_default<S: AsRef<str>>(msg: S, default: Self) -> Self {
        let msg = format!("{} (default={})", msg.as_ref(), default.display());
        prompt_path_opt(msg).unwrap_or(default)
    }
}

/// Specialized `bool` prompter that supports yes/no (y/n) values
impl Promptable for bool {
    /// Prompt for `bool` represented as `true/false`, `yes/no`, or `y/n` input
    ///
    /// The prompt will display the options: `(y/n)`
    ///
    /// ```no_run
    /// use promptly::Promptable;
    /// bool::prompt("Do you accept the terms?");
    /// ```
    fn prompt<S: AsRef<str>>(msg: S) -> Self {
        prompt_bool(msg)
    }

    /// Prompt for optional `bool` input. Empty input returns `None`.
    ///
    /// The prompt will display the options: `(y/n)`
    ///
    /// ```no_run
    /// use promptly::Promptable;
    /// bool::prompt_opt("Did you even read this question?");
    /// ```
    fn prompt_opt<S: AsRef<str>>(msg: S) -> Option<Self> {
        prompt_bool_opt(msg)
    }

    /// Prompt for optional `bool` input. Empty input returns `None`.
    ///
    /// The prompt will also display the options: `(Y/n)` or `(y/N)` depending on the default
    ///
    /// ```no_run
    /// use promptly::Promptable;
    /// bool::prompt_default("Would you like to send us money?", true);
    /// ```
    fn prompt_default<S: AsRef<str>>(msg: S, default: Self) -> Self {
        let msg = match default {
            true => format!("{} (Y/n)", msg.as_ref()),
            false => format!("{} (y/N)", msg.as_ref()),
        };
        prompt_bool_opt(msg).unwrap_or(default)
    }
}

impl<P: Promptable> Promptable for Option<P> {
    fn prompt<S: AsRef<str>>(msg: S) -> Self {
        P::prompt_opt(msg)
    }
    fn prompt_opt<S: AsRef<str>>(_msg: S) -> Option<Self> {
        unimplemented!(
            "prompt_opt is not implemented for Option types as it would return Option<Option<T>>"
        );
    }
    fn prompt_default<S: AsRef<str>>(msg: S, default: Self) -> Self {
        P::prompt_opt(msg).or(default)
    }
}

/// Optinionated wrapper around rustyline to prompt for strings
pub struct Prompter<C: Completer> {
    editor: Editor<C>,
}

impl Prompter<()> {
    pub fn new() -> Prompter<()> {
        Prompter {
            editor: Editor::new(),
        }
    }
}

impl<C> Prompter<C>
where
    C: Completer,
{
    pub fn with_completer(completer: C) -> Prompter<C> {
        let mut editor = Editor::new();
        editor.set_completer(Some(completer));
        Prompter { editor }
    }

    pub fn prompt_once<S: AsRef<str>>(&mut self, msg: S) -> String {
        match self.editor.readline(&format!("{}: ", msg.as_ref())) {
            Ok(line) => line.trim().to_owned(),
            Err(err) => {
                match err {
                    ReadlineError::Interrupted => (),
                    _ => println!("Readline error: {}", err),
                }
                ::std::process::exit(1);
            }
        }
    }

    /// Prompts once but returns `None` for empty input
    pub fn prompt_opt<S: AsRef<str>>(&mut self, msg: S) -> Option<String> {
        let val = self.prompt_once(msg);
        if val.is_empty() {
            return None;
        }
        Some(val)
    }

    /// Prompts until a non-empty value is provided
    pub fn prompt_nonempty<S: AsRef<str>>(&mut self, msg: S) -> String {
        let mut val;
        val = self.prompt_opt(&msg);
        while val.is_none() {
            eprintln!("Value is required.");
            val = self.prompt_opt(&msg);
        }
        val.unwrap()
    }

    /// Prompts with custom handler to transform input
    pub fn prompt_then<S, F, U>(&mut self, msg: S, handler: F) -> U
    where
        S: AsRef<str>,
        F: Fn(String) -> ::std::result::Result<U, String>,
    {
        let mut val = handler(self.prompt_once(&msg));
        while let Err(e) = val {
            eprintln!("{}", e);
            val = handler(self.prompt_once(&msg));
        }
        val.unwrap()
    }
}

/*
 * Prompt helpers
 */

fn prompt_bool<S: AsRef<str>>(msg: S) -> bool {
    Prompter::new().prompt_then(msg, |s| match &*s.to_lowercase() {
        "true" | "yes" | "y" => Ok(true),
        "false" | "no" | "n" => Ok(false),
        s => Err(format!("Could not parse {} as bool.", s)),
    })
}

fn prompt_bool_opt<S: AsRef<str>>(msg: S) -> Option<bool> {
    Prompter::new().prompt_then(msg, |s| match &*s.to_lowercase().trim() {
        "" => Ok(None),
        "true" | "yes" | "y" => Ok(Some(true)),
        "false" | "no" | "n" => Ok(Some(false)),
        s => Err(format!("Could not parse {} as bool.", s)),
    })
}

fn prompt_path<S: AsRef<str>>(msg: S) -> PathBuf {
    let completer = FilenameCompleter::new();
    let s = Prompter::with_completer(completer).prompt_nonempty(msg);
    PathBuf::from(path_expand(s))
}

fn prompt_path_opt<S: AsRef<str>>(msg: S) -> Option<PathBuf> {
    let completer = FilenameCompleter::new();
    Prompter::with_completer(completer)
        .prompt_opt(msg)
        .map(path_expand)
        .map(PathBuf::from)
}

fn prompt_parse<T, S>(msg: S) -> T
where
    T: FromStr,
    <T as FromStr>::Err: ::std::error::Error,
    S: AsRef<str>,
{
    Prompter::new().prompt_then(msg, |s| T::from_str(s.as_ref()).map_err(|e| e.to_string()))
}

fn prompt_parse_opt<T, S>(msg: S) -> Option<T>
where
    T: FromStr,
    <T as FromStr>::Err: ::std::error::Error,
    S: AsRef<str>,
{
    Prompter::new().prompt_then(msg, |s| match s.trim() {
        "" => Ok(None),
        _ => match T::from_str(s.as_ref()) {
            Ok(n) => Ok(Some(n)),
            Err(e) => Err(e.to_string()),
        },
    })
}

fn path_expand(s: String) -> String {
    if s.starts_with("~") {
        if let Ok(home) = env::var("HOME") {
            return s.replacen("~", &home, 1);
        }
    }
    s
}
