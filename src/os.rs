use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::any::Any;

pub use anyhow::Result;

pub use crate::interpreter::path::Path;

#[derive(Debug, Clone)]
pub struct MachineInfo {
    pub system: String,
    pub cpu: String,
    pub endian: String,
}

#[derive(Debug, Clone)]
pub struct TryCompileOutput {
    pub success: bool,
    pub artifact: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct RunCommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub returncode: i64,
}

pub struct TempDir {
    path: Path,
    _opaque: Box<dyn Any>,
}

impl TempDir {
    pub fn new(path: Path, opaque: impl Any) -> Self {
        Self {
            path,
            _opaque: Box::new(opaque),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

pub struct CompilerInfo {
    pub bin: Path,
    pub flags: Vec<String>,
}

pub trait Os: 'static {
    // printing
    fn print(&self, msg: &str);

    // env
    fn get_env(&self, key: &str) -> Option<String>;
    fn build_machine(&self) -> Result<MachineInfo>;
    fn host_machine(&self) -> Result<MachineInfo>;
    fn default_prefix(&self) -> Result<Path>;

    // fs
    fn is_file(&self, path: &Path) -> Result<bool>;
    fn is_dir(&self, path: &Path) -> Result<bool>;
    fn exists(&self, path: &Path) -> Result<bool>;
    fn read_file(&self, path: &Path) -> Result<Vec<u8>>;
    fn write_file(&self, path: &Path, data: &[u8]) -> Result<()>;

    fn tempdir(&self) -> Result<TempDir>;

    // compiler
    fn get_compiler(&self, lang: &str) -> Result<CompilerInfo>;

    // misc
    fn find_program(&self, name: &Path, pwd: &Path) -> Result<Path>;
    fn run_command(&self, cmd: &Path, args: &[&str]) -> Result<RunCommandOutput>;
}
