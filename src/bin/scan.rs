//! Scan a project and print a summary, without opening a window.
//! Handy for checking the ignore rules on a new project:
//!
//!   cargo run --release --bin scan -- /path/to/project [ignored/path ...]
//!
//! Extra paths are read the way a click on an ignored box reads them.
//!
//! `--check-fonts` instead checks that the fonts the app draws with sit where it
//! will look for them, which nothing in a build can prove on its own.

// Reuse the app's scanner and tree code by pointing at the same files.
#[allow(dead_code)]
#[path = "../history.rs"]
mod history;
#[allow(dead_code)]
#[path = "../model.rs"]
mod model;
#[allow(dead_code)]
#[path = "../scan.rs"]
mod scan;

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

fn check_fonts() {
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

fn main() {
    if std::env::args().any(|arg| arg == "--check-fonts") {
        check_fonts();
        return;
    }
    let path = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let root = path.canonicalize().unwrap_or(path);
    let started = std::time::Instant::now();
    let project = match scan::scan_project(&root) {
        Ok(project) => project,
        Err(err) => {
            eprintln!("scan failed: {err}");
            std::process::exit(1);
        }
    };
    let used_git = project.used_git;
    let mut tree = model::Tree::new("root");
    for entry in project.entries {
        tree.insert(entry);
    }
    tree.update_weights(true);
    tree.layout(model::R { x: 0.0, y: 0.0, w: 1000.0, h: 700.0 });

    let top = &tree.nodes[0];
    println!("{}", root.display());
    println!(
        "{} files, {} lines, {} tree nodes, {:.2}s, ignore rules from git: {}",
        top.total_files,
        top.total_lines,
        tree.nodes.len(),
        started.elapsed().as_secs_f64(),
        used_git
    );
    let ghosts: Vec<&str> =
        tree.nodes.iter().filter(|n| matches!(n.kind, model::Kind::Ghost { .. })).map(|n| n.path.as_str()).collect();
    println!("{} ignored entries (shown collapsed), first 15:", ghosts.len());
    for g in ghosts.iter().take(15) {
        println!("  {g}");
    }
    let mut biggest: Vec<&model::Node> = top.children.iter().map(|&c| &tree.nodes[c]).collect();
    biggest.sort_by_key(|n| std::cmp::Reverse(n.total_lines));
    println!("largest top-level entries (lines, map size of 1000x700):");
    for n in biggest.iter().take(12) {
        println!("  {:>10}  {:>6.1} x {:<6.1}  {}", n.total_lines, n.rect.w, n.rect.h, n.name);
    }

    let started = std::time::Instant::now();
    match history::load(&root) {
        Ok(h) => {
            tree.apply_history(&h);
            let mut files: Vec<&model::Node> = tree.nodes.iter().filter(|n| n.commits > 0 && n.is_file()).collect();
            files.sort_by_key(|n| std::cmp::Reverse(n.commits));
            println!(
                "git history: {} commits, {} files with history, {:.2}s. Most changed:",
                h.commits_read,
                files.len(),
                started.elapsed().as_secs_f64()
            );
            for n in files.iter().take(5) {
                println!("  {:>5} commits  {}", n.commits, n.path);
            }
            tree.compute_heat(model::ColorMode::Churn);
            let hot = tree.nodes.iter().filter(|n| n.heat >= 0.99).count();
            println!("  heat computed, {hot} files at the top rank");
        }
        Err(err) => println!("git history: {err}"),
    }
    let matches = tree.search("text input");
    println!("search \"text input\": {} matches, first: {:?}", matches.len(), matches.iter().take(3).map(|&i| &tree.nodes[i].path).collect::<Vec<_>>());

    for rel in std::env::args().skip(2) {
        let started = std::time::Instant::now();
        let entries = scan::scan_ignored(&root, &rel);
        let Some(index) = tree.find(&rel) else {
            println!("{rel}: not an ignored entry in the tree");
            continue;
        };
        let count = entries.len();
        tree.graft(index, entries);
        tree.update_weights(true);
        tree.layout(model::R { x: 0.0, y: 0.0, w: 1000.0, h: 700.0 });
        let n = &tree.nodes[index];
        println!(
            "expanded {rel}: {count} files read in {:.2}s, {} lines, map size {:.1} x {:.1}",
            started.elapsed().as_secs_f64(),
            n.total_lines,
            n.rect.w,
            n.rect.h
        );
    }
}
