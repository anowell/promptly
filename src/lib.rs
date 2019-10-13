#![cfg_attr(feature="nightly", feature(specialization))]

//! Simply call `prompt` or `prompt_default` to prompt for any `Promptable` type:
//!
//! ## Examples
//!
//! ```no_run
//! # use std::path::PathBuf;
//! use promptly::{prompt, prompt_default};
//!
//! // Prompt until a non-empty string is provided
//! let name: String = prompt("Enter your name");
//!
//! // Prompt for other `FromStr` types
//! let age: u32 = prompt("Enter your age");
//!
//! // Prompt for optional paths with path completion. Returns `None` if empty input.
//! let photo: Option<PathBuf> = prompt("Enter a path to a profile picture");
//!
//! // Prompt Y/n with a default value when input is empty
//! let fallback = prompt_default("Would you like to receive marketing emails", true);
//! ```
//!
//! ## Errors
//! If readline fails to read from stdin, this call will exit the process with an exit code of `1`.
//! All other errors just result in re-prompting.

extern crate rustyline;
#[cfg(feature = "url")]
extern crate url;

use rustyline::completion::{Completer, FilenameCompleter};
use rustyline::{error::ReadlineError, Editor};
use std::env;
use std::path::PathBuf;
use std::str::FromStr;

#[cfg(feature = "nightly")]
use std::fmt::Display;


/// Prompt until input can be parsed as `T`.
///
/// Empty string input causes a re-prompt (including for `String`)
/// except when `T` is an `Option`-wrapped type.
///
/// ## Examples
///
/// ```no_run
/// # use std::path::PathBuf;
/// use promptly::{prompt, prompt_default};
///
/// // Prompt until a non-empty string is provided
/// let name: String = prompt("Enter your name");
///
/// // Prompt for an optional string
/// let name: Option<String> = prompt("Enter your name (optional)");
///
/// // Prompt for other `FromStr` types
/// let age: u32 = prompt("Enter your age");
///
/// // Prompt for optional paths with path completion. Returns `None` if empty input.
/// let photo: Option<PathBuf> = prompt("Enter a path to a profile picture");
/// ```
///
/// ## Errors
/// If readline fails to read from stdin, this call will exit the process with an exit code of `1`.
/// All other errors just result in re-prompting.
pub fn prompt<T, S>(msg: S) -> T
where T: Promptable,
      S: AsRef<str>,
{
    T::prompt(msg)
}

/// Prompt until input can be parsed as `T`, returning the `default` for empty input.
///
/// ## Examples
///
/// ```no_run
/// # use std::net::Ipv4Addr;
/// # use std::path::PathBuf;
/// use promptly::{prompt, prompt_default};
///
/// // Prompt Y/n with a default value when input is empty
/// let fallback = prompt_default("Would you like to receive marketing emails", true);
///
/// // Prompt for a string with default
/// let fav_lang = prompt_default("Enter you favorite programming language", "Rust".to_string());
///
/// // Prompt for other `FromStr` types
/// let local_ip = prompt_default("Enter your local IP", Ipv4Addr::new(127, 0, 0, 1));
/// ```
///
/// ## Errors
/// If readline fails to read from stdin, this call will exit the process with an exit code of `1`.
/// All other errors just result in re-prompting.
pub fn prompt_default<T, S>(msg: S, default: T) -> T
where T: Promptable,
      S: AsRef<str>,
{
    T::prompt_default(msg, default)
}

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
        let msg = if default {
            format!("{} (Y/n)", msg.as_ref())
        } else {
            format!("{} (y/N)", msg.as_ref())
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

macro_rules! impl_promptable_from_str {
    ($t:ty) => {
        impl Promptable for $t {
            fn prompt<S: AsRef<str>>(msg: S) -> Self {
                prompt_parse(msg)
            }

            fn prompt_opt<S: AsRef<str>>(msg: S) -> Option<Self> {
                prompt_parse_opt(msg)
            }

            fn prompt_default<S: AsRef<str>>(msg: S, default: Self) -> Self {
                let msg = format!("{} (default={})", msg.as_ref(), default);
                prompt_parse_opt(msg).unwrap_or(default)
            }
        }
    }
}

impl_promptable_from_str!(u32);
impl_promptable_from_str!(u64);
impl_promptable_from_str!(i32);
impl_promptable_from_str!(i64);
impl_promptable_from_str!(f32);
impl_promptable_from_str!(f64);
impl_promptable_from_str!(::std::net::IpAddr);
impl_promptable_from_str!(::std::net::Ipv4Addr);
impl_promptable_from_str!(::std::net::Ipv6Addr);

#[cfg(feature = "url")]
impl_promptable_from_str!(::url::Url);

#[cfg(feature = "nightly")]
/// Blanket impl for `FromStr` types. Re-prompts until `FromStr` parsing succeeds.
impl<T> Promptable for T
where
    T: FromStr + Display,
    <T as FromStr>::Err: ::std::error::Error
{
    /// Prompt until the input parses into the specified type
    ///
    /// ```no_run
    /// use promptly::Promptable;
    /// u32::prompt("Enter your age");
    /// ```
    default fn prompt<S: AsRef<str>>(msg: S) -> Self {
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
    default fn prompt_opt<S: AsRef<str>>(msg: S) -> Option<Self> {
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
    default fn prompt_default<S: AsRef<str>>(msg: S, default: Self) -> Self {
        let msg = format!("{} (default={})", msg.as_ref(), default);
        prompt_parse_opt(msg).unwrap_or(default)
    }
}

/// Optinionated wrapper around rustyline to prompt for strings
pub struct Prompter<C: Completer> {
    editor: Editor<C>,
    err_handler: Box<dyn Fn(ReadlineError)>, // TODO: closure should return Never type
}

impl Prompter<()> {
    pub fn new() -> Prompter<()> {
        Prompter {
            editor: Editor::new(),
            err_handler: Box::new(default_err_handler)
        }
    }
}

fn default_err_handler(err: ReadlineError) {
    match err {
        ReadlineError::Interrupted => (),
        _ => println!("Readline error: {}", err),
    }
    ::std::process::exit(1);
}

impl<C> Prompter<C>
where
    C: Completer,
{
    pub fn with_completer(completer: C) -> Prompter<C> {
        let mut editor = Editor::new();
        editor.set_completer(Some(completer));
        Prompter {
            editor,
            err_handler: Box::new(default_err_handler)
        }
    }

    pub fn on_error<F: Fn(ReadlineError) + 'static>(mut self, handler: F) {
        self.err_handler = Box::new(handler);
    }

    pub fn prompt_once<S: AsRef<str>>(&mut self, msg: S) -> String {
        match self.editor.readline(&format!("{}: ", msg.as_ref())) {
            Ok(line) => line.trim().to_owned(),
            Err(err) => {
                (self.err_handler)(err);
                unreachable!("Prompter's on_error handler should never return")
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
