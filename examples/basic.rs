use promptly::{prompt, prompt_default, prompt_opt, Promptable};
use std::error::Error;
use std::path::PathBuf;

fn test<P: Promptable>(ty: &str, default: P) {
    let _ = prompt::<P, _>(ty);
    let _ = prompt_opt::<P, _>(format!("Option<{}>", ty));
    let _ = prompt_default::<P, _>(ty, default);
}

fn main() -> Result<(), Box<dyn Error>> {
    test::<String>("String", "DefaultValue".to_string());
    test::<u32>("u32", 0);
    test::<bool>("u32", false);
    test::<PathBuf>("PathBuf", PathBuf::from("/home"));

    Ok(())
}
