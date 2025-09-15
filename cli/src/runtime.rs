use std::env::consts::{ARCH, OS};
use std::path::Path;
use std::process::Command;
use std::{env, fs};

use anyhow::bail;
use picomeson::path::Path as OsPath;
use picomeson::runtime::{self, CompilerInfo};
use tempfile::tempdir;

pub struct Sandbox;

const ENDIAN: &str = if cfg!(target_endian = "little") {
    "little"
} else {
    "big"
};

impl runtime::Runtime for Sandbox {
    fn print(&self, msg: &str) {
        println!("{}", msg);
    }

    fn get_env(&self, _key: &str) -> Option<String> {
        None
    }

    fn build_machine(&self) -> runtime::Result<runtime::MachineInfo> {
        Ok(runtime::MachineInfo {
            system: OS.into(),
            cpu: ARCH.into(),
            endian: ENDIAN.into(),
        })
    }

    fn host_machine(&self) -> runtime::Result<runtime::MachineInfo> {
        let system = env::var("CARGO_CFG_TARGET_OS").unwrap_or(OS.into());
        let cpu = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or(ARCH.into());
        let endian = env::var("CARGO_CFG_TARGET_ENDIAN").unwrap_or(ENDIAN.into());
        Ok(runtime::MachineInfo {
            system,
            cpu,
            endian,
        })
    }

    fn is_file(&self, path: &OsPath) -> runtime::Result<bool> {
        Ok(Path::new(path.as_ref()).is_file())
    }
    fn is_dir(&self, path: &OsPath) -> runtime::Result<bool> {
        Ok(Path::new(path.as_ref()).is_dir())
    }
    fn exists(&self, path: &OsPath) -> runtime::Result<bool> {
        Ok(Path::new(path.as_ref()).exists())
    }
    fn read_file(&self, path: &OsPath) -> runtime::Result<Vec<u8>> {
        Ok(fs::read(path.as_ref())?)
    }
    fn write_file(&self, path: &OsPath, data: &[u8]) -> runtime::Result<()> {
        Ok(fs::write(path.as_ref(), data)?)
    }
    fn tempdir(&self) -> runtime::Result<runtime::TempDir> {
        let dir = tempdir()?;
        let path = dir.path().to_string_lossy().into_owned();
        let path = OsPath::from(path);
        Ok(runtime::TempDir::new(path, dir))
    }

    fn get_compiler(&self, lang: &str) -> runtime::Result<runtime::CompilerInfo> {
        match lang {
            "c" => Ok(CompilerInfo {
                bin: OsPath::from("cc"),
                flags: vec![],
            }),
            _ => bail!("Unsupported language: {lang}"),
        }
    }

    fn find_program(&self, name: &OsPath, _cwd: &OsPath) -> runtime::Result<OsPath> {
        bail!("Not found: {}", name.as_ref());
    }

    fn run_command(
        &self,
        cmd: &OsPath,
        args: &[&str],
    ) -> runtime::Result<runtime::RunCommandOutput> {
        //eprintln!("Running command: {} {:?}", cmd.as_ref(), args);

        if cmd.as_ref() != "cc" {
            bail!("Unsupported command: {}", cmd.as_ref());
        }

        let output = Command::new(cmd.as_ref()).args(args).output()?;

        Ok(picomeson::runtime::RunCommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            returncode: output.status.code().unwrap_or(-1) as i64,
        })
    }
}
