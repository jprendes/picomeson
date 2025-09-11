use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::any::Any;

pub use anyhow::Result;

pub use crate::interpreter::path::Path;

/// Information about a machine's architecture and system
#[derive(Debug, Clone)]
pub struct MachineInfo {
    /// The operating system name (e.g., "linux", "windows", "darwin")
    pub system: String,
    /// The CPU architecture (e.g., "x86_64", "aarch64", "arm")
    pub cpu: String,
    /// The endianness of the system ("little" or "big")
    pub endian: String,
}

/// Result of attempting to compile source code
#[derive(Debug, Clone)]
pub struct TryCompileOutput {
    /// Whether the compilation succeeded
    pub success: bool,
    /// The compiled artifact bytes if successful
    pub artifact: Vec<u8>,
}

/// Output from running a command
#[derive(Debug, Clone)]
pub struct RunCommandOutput {
    /// The standard output from the command
    pub stdout: String,
    /// The standard error from the command
    pub stderr: String,
    /// The exit code of the command (0 typically means success)
    pub returncode: i64,
}

/// A temporary directory that will be cleaned up when dropped
pub struct TempDir {
    path: Path,
    _opaque: Box<dyn Any>,
}

impl TempDir {
    /// Creates a new temporary directory wrapper
    ///
    /// # Arguments
    /// * `path` - The path to the temporary directory
    /// * `opaque` - Platform-specific handle for cleanup
    pub fn new(path: Path, opaque: impl Any) -> Self {
        Self {
            path,
            _opaque: Box::new(opaque),
        }
    }

    /// Returns the path to the temporary directory
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Information about a compiler
pub struct CompilerInfo {
    /// Path to the compiler executable
    pub bin: Path,
    /// Default flags to pass to the compiler
    pub flags: Vec<String>,
}

/// Operating system abstraction layer
///
/// This trait provides platform-agnostic access to OS functionality
/// needed by the Meson build system implementation.
pub trait Os: 'static {
    /// Prints a message to the console
    ///
    /// # Arguments
    /// * `msg` - The message to print
    fn print(&self, msg: &str);
    
    /// Gets the value of an environment variable
    ///
    /// # Arguments
    /// * `key` - The name of the environment variable
    ///
    /// # Returns
    /// The value of the environment variable if it exists
    fn get_env(&self, key: &str) -> Option<String>;
    
    /// Gets information about the build machine
    ///
    /// The build machine is the system where the build is being performed.
    ///
    /// # Returns
    /// Machine information including system, CPU, and endianness
    fn build_machine(&self) -> Result<MachineInfo>;
    
    /// Gets information about the host machine
    ///
    /// The host machine is the system where the built binaries will run.
    /// This may differ from the build machine when cross-compiling.
    ///
    /// # Returns
    /// Machine information including system, CPU, and endianness
    fn host_machine(&self) -> Result<MachineInfo>;
    
    /// Gets the default installation prefix for the current platform
    ///
    /// # Returns
    /// The default prefix path (e.g., "/usr/local" on Unix-like systems)
    fn default_prefix(&self) -> Result<Path>;
    
    /// Checks if a path points to a regular file
    ///
    /// # Arguments
    /// * `path` - The path to check
    ///
    /// # Returns
    /// `true` if the path is a file, `false` otherwise
    fn is_file(&self, path: &Path) -> Result<bool>;
    
    /// Checks if a path points to a directory
    ///
    /// # Arguments
    /// * `path` - The path to check
    ///
    /// # Returns
    /// `true` if the path is a directory, `false` otherwise
    fn is_dir(&self, path: &Path) -> Result<bool>;
    
    /// Checks if a path exists
    ///
    /// # Arguments
    /// * `path` - The path to check
    ///
    /// # Returns
    /// `true` if the path exists, `false` otherwise
    fn exists(&self, path: &Path) -> Result<bool>;
    
    /// Reads the contents of a file
    ///
    /// # Arguments
    /// * `path` - The path to the file to read
    ///
    /// # Returns
    /// The contents of the file as a byte vector
    fn read_file(&self, path: &Path) -> Result<Vec<u8>>;
    
    /// Writes data to a file
    ///
    /// Creates the file if it doesn't exist, overwrites if it does.
    ///
    /// # Arguments
    /// * `path` - The path where the file should be written
    /// * `data` - The data to write to the file
    fn write_file(&self, path: &Path, data: &[u8]) -> Result<()>;

    /// Creates a new temporary directory
    ///
    /// The directory will be automatically cleaned up when the returned
    /// `TempDir` is dropped.
    ///
    /// # Returns
    /// A handle to the temporary directory
    fn tempdir(&self) -> Result<TempDir>;
    
    /// Gets compiler information for a specific language
    ///
    /// # Arguments
    /// * `lang` - The language identifier (e.g., "c", "cpp", "rust")
    ///
    /// # Returns
    /// Information about the compiler including its path and default flags
    fn get_compiler(&self, lang: &str) -> Result<CompilerInfo>;
    
    /// Finds a program in the system PATH or at a specific location
    ///
    /// # Arguments
    /// * `name` - The name or path of the program to find
    /// * `pwd` - The current working directory for relative path resolution
    ///
    /// # Returns
    /// The absolute path to the program if found
    fn find_program(&self, name: &Path, pwd: &Path) -> Result<Path>;
    
    /// Runs a command and captures its output
    ///
    /// # Arguments
    /// * `cmd` - The path to the command to run
    /// * `args` - The arguments to pass to the command
    ///
    /// # Returns
    /// The output from the command including stdout, stderr, and return code
    fn run_command(&self, cmd: &Path, args: &[&str]) -> Result<RunCommandOutput>;
}
