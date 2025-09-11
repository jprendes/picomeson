use std::env::consts::{ARCH, OS};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use anyhow::bail;
use picomeson::os;
use tempfile::tempdir;

pub struct Os;

const ENDIAN: &str = if cfg!(target_endian = "little") {
    "little"
} else {
    "big"
};

pub const PREFIX: &str = if cfg!(windows) { "C:\\" } else { "/usr/local" };

impl os::Os for Os {
    fn print(&self, msg: &str) {
        println!("{}", msg);
    }

    fn get_env(&self, key: &str) -> Option<String> {
        env::var(key).ok()
    }

    fn build_machine(&self) -> os::Result<os::MachineInfo> {
        Ok(os::MachineInfo {
            system: OS.into(),
            cpu: ARCH.into(),
            endian: ENDIAN.into(),
        })
    }

    fn host_machine(&self) -> os::Result<os::MachineInfo> {
        let system = env::var("CARGO_CFG_TARGET_OS").unwrap_or(OS.into());
        let cpu = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or(ARCH.into());
        let endian = env::var("CARGO_CFG_TARGET_ENDIAN").unwrap_or(ENDIAN.into());
        Ok(os::MachineInfo {
            system,
            cpu,
            endian,
        })
    }

    fn seppath(&self) -> os::Result<String> {
        Ok(env::join_paths(["", ""])?.to_string_lossy().into_owned())
    }

    fn join_paths(&self, paths: &[&str]) -> os::Result<String> {
        let mut path = PathBuf::new();
        for p in paths {
            path.push(p);
        }
        Ok(path.to_string_lossy().into_owned())
    }

    fn is_file(&self, path: &str) -> os::Result<bool> {
        println!("Checking if path is a file: {}", path);
        Ok(Path::new(path).is_file())
    }
    fn is_dir(&self, path: &str) -> os::Result<bool> {
        Ok(Path::new(path).is_dir())
    }
    fn exists(&self, path: &str) -> os::Result<bool> {
        Ok(Path::new(path).exists())
    }
    fn read_file(&self, path: &str) -> os::Result<Vec<u8>> {
        Ok(fs::read(path)?)
    }
    fn write_file(&self, path: &str, data: &[u8]) -> os::Result<()> {
        Ok(fs::write(path, data)?)
    }
    fn tempdir(&self) -> os::Result<os::TempDir> {
        let dir = tempdir()?;
        let path = dir.path().to_string_lossy().into_owned();
        Ok(os::TempDir::new(path, dir))
    }

    fn get_compiler(&self, lang: &str) -> os::Result<Vec<String>> {
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
            _ => bail!("Unsupported language: {lang}"),
        }
    }

    fn find_program(&self, name: &str, cwd: &str) -> os::Result<String> {
        let cwd = env::current_dir()?.join(cwd);
        let path = self.get_env("PATH");

        let path = which::which_in(name, path, cwd)?;

        Ok(path.to_string_lossy().into_owned())
    }

    fn run_command(&self, cmd: &[&str]) -> os::Result<os::RunCommandOutput> {
        if cmd.is_empty() {
            bail!("Command is empty");
        }

        let output = Command::new(cmd[0]).args(&cmd[1..]).output()?;

        Ok(picomeson::os::RunCommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            returncode: output.status.code().unwrap_or(-1) as i64,
        })
    }

    fn default_prefix(&self) -> os::Result<String> {
        Ok(PREFIX.into())
    }
}
