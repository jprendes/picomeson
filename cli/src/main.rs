use std::{env, process};

mod os;

use os::OsEnv;

fn main() -> anyhow::Result<()> {
    let dir_path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: meson_parser <path_to_project_dir>");
        process::exit(1);
    });

    picomeson::Meson::new(OsEnv).build(&dir_path)?;

    Ok(())
}
