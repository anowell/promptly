# promptly

A simple, opinionated prompting library

Features include:

- Re-prompt until valid
- Prompts for several types, and extensible
- Sane handling of escapes via rustyline
- Path completion when prompting for paths
- Dead simple to use. Perhaps too simple.

## Usage

```rust
use promptly::Promptable;

// Prompt until a non-empty string is provided
let name = String::prompt("Enter your name");

// Prompt for other `FromStr` types
let age = u32::prompt("Enter your age");

// Prompt for optional paths with path completion. Returns `None` if empty input.
let photo = PathBuf::prompt_opt("Enter a path to a profile picture");

// Prompt Y/n with a default value when input is empty
let fallback = bool::prompt_default("Would you like to receive marketing emails", true);
```

There are a few other bits exposed that might evolve a bit, but the above really captures the gist of it.
