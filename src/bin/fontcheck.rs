//! Check that the fonts the app draws with are where the app will look for them.
//!
//!   cargo run --release --bin fontcheck
//!
//! Same check as `scan --check-fonts`, but with no makepad dependency, so it
//! builds and runs anywhere - including a container with no GL libraries, and
//! from inside a staged package directory in CI.
//!
//! Why this exists: makepad reads font files at runtime from paths beside the
//! executable (`package_root` + the resource's dependency path). Nothing in a
//! build proves they arrived there, and when they do not the app still starts -
//! it just draws boxes and outlines with no text at all, which is what makes
//! this failure worth a dedicated check.

use std::path::PathBuf;

/// Keep in step with the `crate_resource("self:resources/...")` paths in
/// `src/map_view.rs`: makepad turns `self:` into "<crate name>/<path>" once
/// packaged, and `code-map` becomes `code_map` there.
const FONT_ASSETS: &[&str] = &[
    "code_map/resources/JetBrainsMonoNerdFont-Regular-v1.2.ttf",
    "code_map/resources/MiSans-Medium.ttf",
];

/// Returns the first directory that has all of `FONT_ASSETS`.
///
/// The order mirrors what makepad does with a relative `package_root`: try the
/// current directory first, then the directory of the executable.
fn locate_fonts() -> Option<PathBuf> {
    let mut roots = vec![PathBuf::from(".")];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            roots.push(dir.to_path_buf());
        }
    }
    roots.into_iter().find(|root| FONT_ASSETS.iter().all(|asset| root.join(asset).is_file()))
}

fn main() {
    if let Some(root) = locate_fonts() {
        println!("fonts resolved from {}", root.canonicalize().unwrap_or(root).display());
        for asset in FONT_ASSETS {
            println!("  ok  {asset}");
        }
        return;
    }
    eprintln!("fonts were not found next to the executable. Looked for:");
    for asset in FONT_ASSETS {
        eprintln!("  {asset}");
    }
    eprintln!("The app would draw boxes and outlines but no text. Ship the resources/");
    eprintln!("directory as code_map/resources/ alongside the binary.");
    std::process::exit(1);
}
