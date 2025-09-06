extern crate alloc;

mod interpreter;
mod parser;

fn main() {
    let file_path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: meson_parser <path_to_meson_file>");
        std::process::exit(1);
    });

    let file_path = std::path::Path::new(&file_path);
    let dir = file_path.parent().unwrap_or_else(|| {
        eprintln!("Invalid file path");
        std::process::exit(1);
    });
    let file_name = file_path.file_name().unwrap_or_else(|| {
        eprintln!("Invalid file name");
        std::process::exit(1);
    });

    std::env::set_current_dir(dir).expect("Failed to change directory");

    let builtin_options = include_str!("builtin-options.txt");
    let meson_options = std::fs::read_to_string("meson_options.txt").unwrap_or_default();
    let meson_code = std::fs::read_to_string(file_name).expect("Failed to read Meson file");

    let meson_code = format!(
        "
{builtin_options}
{meson_options}
{meson_code}
"
    );

    match parser::parse_meson_file(&meson_code) {
        Ok(statements) => {
            if let Err(e) = interpreter::run_interpreter(statements) {
                eprintln!("Interpreter error: {}", e);
                std::process::exit(1);
            }
        }
        Err(e) => eprintln!("Error parsing Meson file: {}", e),
    }
}
