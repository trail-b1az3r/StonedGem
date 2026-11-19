//! `zsharp-docs [OUT]` — build the Z#++ documentation site (default:
//! `target/zsharp-docs`). Run from anywhere inside the repository.

use std::path::{Path, PathBuf};

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| root.join("target/zsharp-docs"));
    match stonedgem_zsharp_docs::build(&root, &out) {
        Ok(r) => println!(
            "built {} pages into {} ({} examples run, {} checked)",
            r.pages,
            out.display(),
            r.examples_run,
            r.examples_checked
        ),
        Err(e) => {
            eprintln!("zsharp-docs: {e}");
            std::process::exit(1);
        }
    }
}
