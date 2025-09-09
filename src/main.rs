use std::path::{Path, PathBuf};
use std::{env, fs};

extern crate alloc;

mod interpreter;
mod os;
mod parser;

struct OsEnv;

impl os::Os for OsEnv {
    fn get_env(&self, key: &str) -> Option<String> {
        env::var(key).ok()
    }

    fn host(&self) -> os::MachineInfo {
        os::MachineInfo {
            system: env::consts::OS.into(),
            cpu: env::consts::ARCH.into(),
            endian: if cfg!(target_endian = "little") {
                "little".into()
            } else {
                "big".into()
            },
        }
    }

    fn target(&self) -> os::MachineInfo {
        os::MachineInfo {
            system: env::var("CARGO_CFG_TARGET_OS").unwrap_or_default(),
            cpu: env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default(),
            endian: if env::var("CARGO_CFG_TARGET_ENDIAN").unwrap_or_default() == "big" {
                "big".into()
            } else {
                "little".into()
            },
        }
    }

    fn join_paths(&self, paths: &[&str]) -> String {
        let mut path = PathBuf::new();
        for p in paths {
            path.push(p);
        }
        path.to_string_lossy().into_owned()
    }

    fn is_file(&self, path: &str) -> Result<bool, String> {
        Ok(Path::new(path).is_file())
    }
    fn is_dir(&self, path: &str) -> Result<bool, String> {
        Ok(Path::new(path).is_dir())
    }
    fn exists(&self, path: &str) -> Result<bool, String> {
        Ok(Path::new(path).exists())
    }
    fn read_file(&self, path: &str) -> Result<Vec<u8>, String> {
        fs::read(path).map_err(|e| e.to_string())
    }

    fn get_compiler(&self, lang: &str) -> Result<Vec<String>, String> {
        match lang {
            "c" => {
                let cc = env::var("CC").unwrap_or_else(|_| "cc".into());
                let cflags = self.get_env("CFLAGS").unwrap_or_default();
                let cflags = cflags.split_whitespace().map(String::from);
                Ok(core::iter::once(cc).chain(cflags).collect())
            }
            "cpp" => {
                let cxx = env::var("CXX").unwrap_or_else(|_| "c++".into());
                let cxxflags = self.get_env("CXXFLAGS").unwrap_or_default();
                let cxxflags = cxxflags.split_whitespace().map(String::from);
                Ok(core::iter::once(cxx).chain(cxxflags).collect())
            }
            _ => Err(format!("Unsupported language: {}", lang)),
        }
    }
}

fn main() {
    let file_path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: meson_parser <path_to_meson_file>");
        std::process::exit(1);
    });

    let file_path = Path::new(&file_path);
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
            if let Err(e) = interpreter::run_interpreter(OsEnv, statements) {
                eprintln!("Interpreter error: {}", e);
                std::process::exit(1);
            }
        }
        Err(e) => eprintln!("Error parsing Meson file: {}", e),
    }
}
