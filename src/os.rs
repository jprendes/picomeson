use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::any::Any;

pub use anyhow::Result;

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
    path: String,
    _opaque: Box<dyn Any>,
}

impl TempDir {
    pub fn new(path: String, opaque: impl Any) -> Self {
        Self {
            path,
            _opaque: Box::new(opaque),
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

pub trait Os: 'static {
    // printing
    fn print(&self, msg: &str);

    // env
    fn get_env(&self, key: &str) -> Option<String>;
    fn build_machine(&self) -> Result<MachineInfo>;
    fn host_machine(&self) -> Result<MachineInfo>;
    fn default_prefix(&self) -> Result<String>;

    // path
    fn join_paths(&self, paths: &[&str]) -> Result<String>;

    // fs
    fn seppath(&self) -> Result<String>;
    fn is_file(&self, path: &str) -> Result<bool>;
    fn is_dir(&self, path: &str) -> Result<bool>;
    fn exists(&self, path: &str) -> Result<bool>;
    fn read_file(&self, path: &str) -> Result<Vec<u8>>;
    fn write_file(&self, path: &str, data: &[u8]) -> Result<()>;

    fn tempdir(&self) -> Result<TempDir>;

    // compiler
    fn get_compiler(&self, lang: &str) -> Result<Vec<String>>;

    // misc
    fn find_program(&self, name: &str, pwd: &str) -> Result<String>;
    fn run_command(&self, cmd: &[&str]) -> Result<RunCommandOutput>;
}
