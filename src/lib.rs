#![cfg_attr(feature = "nightly", feature(specialization))]

//! Simply call `prompt` or `prompt_default` to prompt for any `Promptable` type:
//!
//! ## Examples
//!
//! ```no_run
//! # use std::path::PathBuf;
//! use promptly::{prompt, prompt_opt, prompt_default};
//!
//! // Prompt until a non-empty string is provided
//! let name: String = prompt("Enter your name")?;
//!
//! // Prompt for other `FromStr` types
//! let age: u32 = prompt("Enter your age")?;
//!
//! // Prompt for optional paths with path completion. Returns `None` if empty input.
//! let photo: Option<PathBuf> = prompt_opt("Enter a path to a profile picture")?;
//!
//! // Prompt Y/n with a default value when input is empty
//! let fallback = prompt_default("Would you like to receive marketing emails", true)?;
//!
//! # Result::<_,Box<std::error::Error>>::Ok(())
//! ```

use rustyline::completion::{Completer, FilenameCompleter};
use rustyline::Editor;
use std::env;
use std::path::PathBuf;
use std::str::FromStr;

pub use rustyline::error::ReadlineError;
type Result<T> = std::result::Result<T, ReadlineError>;

#[cfg(feature = "nightly")]
use std::fmt::Display;

/// Prompt until input can be parsed as `T`.
///
/// Empty string input causes a re-prompt (including for `String`)
///
/// ## Examples
///
/// ```no_run
/// # use std::path::PathBuf;
/// use promptly::prompt;
///
/// // Prompt until a non-empty string is provided
/// let name: String = prompt("Enter your name")?;
///
/// // Prompt for other `FromStr` types
/// let age: u32 = prompt("Enter your age")?;
///
/// # Result::<_,Box<std::error::Error>>::Ok(())
/// ```
///
/// ## Errors
/// Returns a `ReadlineError` if readline fails.
/// Input that can't be coerced into the specified type results in re-prompting.
pub fn prompt<T, S>(msg: S) -> Result<T>
where
    T: Promptable,
    S: AsRef<str>,
{
    T::prompt(msg)
}

/// Prompt until input can be parsed as `T`.
///
/// Empty string input results in `None`
///
/// ## Examples
///
/// ```no_run
/// # use std::path::PathBuf;
/// use promptly::prompt_opt;
///
/// // Prompt for an optional string
/// let name: Option<String> = prompt_opt("Enter your name (optional)")?;
///
/// // Prompt for optional paths with path completion. Returns `None` if empty input.
/// let photo: Option<PathBuf> = prompt_opt("Enter a path to a profile picture")?;
///
/// # Result::<_,Box<std::error::Error>>::Ok(())
/// ```
///
/// ## Errors
/// Returns a `ReadlineError` if readline fails.
/// Input that can't be coerced into the specified type results in re-prompting.
pub fn prompt_opt<T, S>(msg: S) -> Result<Option<T>>
where
    T: Promptable,
    S: AsRef<str>,
{
    T::prompt_opt(msg)
}

/// Prompt until input can be parsed as `T`, returning the `default` for empty input.
///
/// ## Examples
///
/// ```no_run
/// # use std::net::Ipv4Addr;
/// # use std::path::PathBuf;
/// use promptly::prompt_default;
///
/// // Prompt Y/n with a default value when input is empty
/// let fallback = prompt_default("Would you like to receive marketing emails", true)?;
///
/// // Prompt for a string with default
/// let fav_lang = prompt_default("Enter you favorite programming language", "Rust".to_string())?;
///
/// // Prompt for other `FromStr` types
/// let local_ip = prompt_default("Enter your local IP", Ipv4Addr::new(127, 0, 0, 1))?;
///
/// # Result::<_,Box<std::error::Error>>::Ok(())
/// ```
///
/// ## Errors
/// Returns a `ReadlineError` if readline fails.
/// Input that can't be coerced into the specified type results in re-prompting.
pub fn prompt_default<T, S>(msg: S, default: T) -> Result<T>
where
    T: Promptable,
    S: AsRef<str>,
{
    T::prompt_default(msg, default)
}

/// A trait for convenient, opinionated prompting
pub trait Promptable: Sized {
    /// Prompts for a value. Re-prompts on invalid and empty input.
    fn prompt<S: AsRef<str>>(msg: S) -> Result<Self>;

    /// Prompts for a value, returning `None` for empty input. Re-prompts on invalid input.
    fn prompt_opt<S: AsRef<str>>(msg: S) -> Result<Option<Self>>;

    /// Prompts for a value with a default value for empty input. Re-prompts on invalid input.
    ///
    /// The default value will be mentioned in the prompt message
    fn prompt_default<S: AsRef<str>>(msg: S, default: Self) -> Result<Self>;
}

#[cfg(feature = "nightly")]
/// Blanket impl for `FromStr` types. Re-prompts until `FromStr` parsing succeeds.
default impl<T> Promptable for T
where
    T: FromStr + Display,
    <T as FromStr>::Err: ::std::error::Error,
{
    /// Prompt until the input parses into the specified type
    ///
    /// ```no_run
    /// use promptly::Promptable;
    /// u32::prompt("Enter your age")?;
    /// # Result::<_,Box<std::error::Error>>::Ok(())
    /// ```
    fn prompt<S: AsRef<str>>(msg: S) -> Result<Self> {
        prompt_parse(msg)
    }

    /// Prompt for an optional, parseable value.
    ///
    /// Returns `None` if empty, otherwise prompts until input parses into specified type.
    ///
    /// ```no_run
    /// # use std::net::IpAddr;
    /// use promptly::Promptable;
    /// IpAddr::prompt_opt("Enter your IP Address (optional)")?;
    /// # Result::<_,Box<std::error::Error>>::Ok(())
    /// ```
    fn prompt_opt<S: AsRef<str>>(msg: S) -> Result<Option<Self>> {
        prompt_parse_opt(msg)
    }

    /// Prompt for a parseable value with a provided fallback value if empty.
    ///
    /// ```no_run
    /// use promptly::Promptable;
    /// u32::prompt_default("Enter the year", 2018)?;
    /// # Result::<_,Box<std::error::Error>>::Ok(())
    /// ```
    ///
    /// Default value is visible in the prompt as: `(default=USA)`
    fn prompt_default<S: AsRef<str>>(msg: S, default: Self) -> Result<Self> {
        let msg = format!("{} (default={})", msg.as_ref(), default);
        prompt_parse_opt(msg).unwrap_or(default)
    }
}

impl Promptable for String {
    /// Prompt until you get a non-empty string
    ///
    /// ```no_run
    /// use promptly::Promptable;
    /// String::prompt("Enter your name")?;
    /// # Result::<_,Box<std::error::Error>>::Ok(())
    /// ```
    fn prompt<S: AsRef<str>>(msg: S) -> Result<Self> {
        Prompter::new().prompt_nonempty(msg)
    }

    /// Prompt for an optional string
    ///
    /// ```no_run
    /// use promptly::Promptable;
    /// String::prompt_opt("Enter your phone number (optional)")?;
    /// # Result::<_,Box<std::error::Error>>::Ok(())
    /// ```
    fn prompt_opt<S: AsRef<str>>(msg: S) -> Result<Option<Self>> {
        Prompter::new().prompt_opt(msg)
    }

    /// Prompt for a string with a provided fallback value if empty.
    ///
    /// ```no_run
    /// use promptly::Promptable;
    /// String::prompt_default("Enter your country", "USA".into())?;
    /// # Result::<_,Box<std::error::Error>>::Ok(())
    /// ```
    ///
    /// Default value is visible in the prompt as: `(default=USA)`
    fn prompt_default<S: AsRef<str>>(msg: S, default: Self) -> Result<Self> {
        let msg = format!("{} (default={})", msg.as_ref(), default);
        Ok(Prompter::new().prompt_opt(msg)?.unwrap_or(default))
    }
}

/// PathBuf prompting will use a path autocompleter
impl Promptable for PathBuf {
    /// Prompt until you get a non-empty path
    fn prompt<S: AsRef<str>>(msg: S) -> Result<Self> {
        prompt_path(msg)
    }
    /// Prompt for an optional path
    fn prompt_opt<S: AsRef<str>>(msg: S) -> Result<Option<Self>> {
        prompt_path_opt(msg)
    }
    /// Prompt for a path with a provided fallback value if empty
    fn prompt_default<S: AsRef<str>>(msg: S, default: Self) -> Result<Self> {
        let msg = format!("{} (default={})", msg.as_ref(), default.display());
        Ok(prompt_path_opt(msg)?.unwrap_or(default))
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
    /// bool::prompt("Do you accept the terms?")?;
    /// # Result::<_,Box<std::error::Error>>::Ok(())
    /// ```
    fn prompt<S: AsRef<str>>(msg: S) -> Result<Self> {
        prompt_bool(msg)
    }

    /// Prompt for optional `bool` input. Empty input returns `None`.
    ///
    /// The prompt will display the options: `(y/n)`
    ///
    /// ```no_run
    /// use promptly::Promptable;
    /// bool::prompt_opt("Did you even read this question?")?;
    /// # Result::<_,Box<std::error::Error>>::Ok(())
    /// ```
    fn prompt_opt<S: AsRef<str>>(msg: S) -> Result<Option<Self>> {
        prompt_bool_opt(msg)
    }

    /// Prompt for optional `bool` input. Empty input returns `None`.
    ///
    /// The prompt will also display the options: `(Y/n)` or `(y/N)` depending on the default
    ///
    /// ```no_run
    /// use promptly::Promptable;
    /// bool::prompt_default("Would you like to send us money?", true)?;
    /// # Result::<_,Box<std::error::Error>>::Ok(())
    /// ```
    fn prompt_default<S: AsRef<str>>(msg: S, default: Self) -> Result<Self> {
        let msg = if default {
            format!("{} (Y/n)", msg.as_ref())
        } else {
            format!("{} (y/N)", msg.as_ref())
        };
        Ok(prompt_bool_opt(msg)?.unwrap_or(default))
    }
}

// Macro to provide Promptable implementations until specialization stabilizes
macro_rules! impl_promptable_from_str {
    ($t:ty) => {
        impl Promptable for $t {
            fn prompt<S: AsRef<str>>(msg: S) -> Result<Self> {
                prompt_parse(msg)
            }

            fn prompt_opt<S: AsRef<str>>(msg: S) -> Result<Option<Self>> {
                prompt_parse_opt(msg)
            }

            fn prompt_default<S: AsRef<str>>(msg: S, default: Self) -> Result<Self> {
                let msg = format!("{} (default={})", msg.as_ref(), default);
                Ok(prompt_parse_opt(msg)?.unwrap_or(default))
            }
        }
    };
}

impl_promptable_from_str!(char);
impl_promptable_from_str!(u8);
impl_promptable_from_str!(u16);
impl_promptable_from_str!(u32);
impl_promptable_from_str!(u64);
impl_promptable_from_str!(u128);
impl_promptable_from_str!(usize);
impl_promptable_from_str!(i8);
impl_promptable_from_str!(i16);
impl_promptable_from_str!(i32);
impl_promptable_from_str!(i64);
impl_promptable_from_str!(i128);
impl_promptable_from_str!(isize);
impl_promptable_from_str!(f32);
impl_promptable_from_str!(f64);
impl_promptable_from_str!(::std::net::IpAddr);
impl_promptable_from_str!(::std::net::Ipv4Addr);
impl_promptable_from_str!(::std::net::Ipv6Addr);
impl_promptable_from_str!(::std::net::SocketAddrV4);
impl_promptable_from_str!(::std::net::SocketAddrV6);
impl_promptable_from_str!(::std::num::NonZeroI128);
impl_promptable_from_str!(::std::num::NonZeroI64);
impl_promptable_from_str!(::std::num::NonZeroI32);
impl_promptable_from_str!(::std::num::NonZeroI16);
impl_promptable_from_str!(::std::num::NonZeroI8);
impl_promptable_from_str!(::std::num::NonZeroIsize);
impl_promptable_from_str!(::std::num::NonZeroU128);
impl_promptable_from_str!(::std::num::NonZeroU64);
impl_promptable_from_str!(::std::num::NonZeroU32);
impl_promptable_from_str!(::std::num::NonZeroU16);
impl_promptable_from_str!(::std::num::NonZeroU8);
impl_promptable_from_str!(::std::num::NonZeroUsize);

#[cfg(feature = "url")]
impl_promptable_from_str!(url::Url);

/// Optinionated wrapper around rustyline to prompt for strings
pub struct Prompter<C: Completer> {
    editor: Editor<C>,
}

impl Prompter<()> {
    pub fn new() -> Prompter<()> {
        Prompter::default()
    }
}

impl Default for Prompter<()> {
    fn default() -> Self {
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

    pub fn prompt_once<S: AsRef<str>>(&mut self, msg: S) -> Result<String> {
        self.editor
            .readline(&format!("{}: ", msg.as_ref()))
            .map(|line| line.trim().to_owned())
    }

    /// Prompts once but returns `None` for empty input
    pub fn prompt_opt<S: AsRef<str>>(&mut self, msg: S) -> Result<Option<String>> {
        let val = self.prompt_once(msg)?;
        if val.is_empty() {
            return Ok(None);
        }
        Ok(Some(val))
    }

    /// Prompts until a non-empty value is provided
    pub fn prompt_nonempty<S: AsRef<str>>(&mut self, msg: S) -> Result<String> {
        let mut val;
        val = self.prompt_opt(&msg)?;
        while val.is_none() {
            eprintln!("Value is required.");
            val = self.prompt_opt(&msg)?;
        }
        Ok(val.unwrap())
    }

    /// Prompts with custom handler to transform input
    pub fn prompt_then<S, F, U>(&mut self, msg: S, handler: F) -> Result<U>
    where
        S: AsRef<str>,
        F: Fn(String) -> ::std::result::Result<U, String>,
    {
        let mut val = handler(self.prompt_once(&msg)?);
        while let Err(e) = val {
            eprintln!("{}", e);
            val = handler(self.prompt_once(&msg)?);
        }
        Ok(val.unwrap())
    }
}

/*
 * Prompt helpers
 */

fn prompt_bool<S: AsRef<str>>(msg: S) -> Result<bool> {
    Prompter::new().prompt_then(msg, |s| match &*s.to_lowercase() {
        "true" | "yes" | "y" => Ok(true),
        "false" | "no" | "n" => Ok(false),
        s => Err(format!("Could not parse {} as bool.", s)),
    })
}

fn prompt_bool_opt<S: AsRef<str>>(msg: S) -> Result<Option<bool>> {
    Prompter::new().prompt_then(msg, |s| match &*s.to_lowercase().trim() {
        "" => Ok(None),
        "true" | "yes" | "y" => Ok(Some(true)),
        "false" | "no" | "n" => Ok(Some(false)),
        s => Err(format!("Could not parse {} as bool.", s)),
    })
}

fn prompt_path<S: AsRef<str>>(msg: S) -> Result<PathBuf> {
    let completer = FilenameCompleter::new();
    let s = Prompter::with_completer(completer).prompt_nonempty(msg)?;
    Ok(PathBuf::from(path_expand(s)))
}

fn prompt_path_opt<S: AsRef<str>>(msg: S) -> Result<Option<PathBuf>> {
    let completer = FilenameCompleter::new();
    Ok(Prompter::with_completer(completer)
        .prompt_opt(msg)?
        .map(path_expand)
        .map(PathBuf::from))
}

fn prompt_parse<T, S>(msg: S) -> Result<T>
where
    T: FromStr,
    <T as FromStr>::Err: ::std::error::Error,
    S: AsRef<str>,
{
    Prompter::new().prompt_then(msg, |s| T::from_str(s.as_ref()).map_err(|e| e.to_string()))
}

fn prompt_parse_opt<T, S>(msg: S) -> Result<Option<T>>
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
    if s.starts_with('~') {
        if let Ok(home) = env::var("HOME") {
            return s.replacen("~", &home, 1);
        }
    }
    s
}
