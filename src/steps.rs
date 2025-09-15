pub use crate::interpreter::builtins::build_target::BuildTarget;
pub use crate::interpreter::builtins::config_data::ConfigureFile;
use crate::path::Path;

/// Build steps abstraction for generating build system output
///
/// This trait defines the interface for different build system backends
/// (e.g., Ninja, Make, etc.) to generate their specific build instructions.
/// Implementations of this trait are responsible for translating high-level
/// build targets and operations into the appropriate format for their
/// respective build systems.
pub trait BuildSteps: 'static {
    /// Generates build instructions for building a static library
    ///
    /// This method should generate the necessary build steps to compile
    /// source files into object files and archive them into a static library.
    ///
    /// # Arguments
    /// * `target` - The build target containing information about sources,
    ///   include directories, compile flags, and output location
    fn build_static_library(&self, target: &BuildTarget);

    /// Generates build instructions for building an executable
    ///
    /// This method should generate the necessary build steps to compile
    /// source files into object files and link them into an executable binary.
    ///
    /// # Arguments
    /// * `target` - The build target containing information about sources,
    ///   include directories, compile flags, link flags, dependencies,
    ///   and output location
    fn build_executable(&self, target: &BuildTarget);

    /// Generates build instructions for generating a file
    ///
    /// This method should generate a build step that writes a file to disk
    /// based on the content of ConfigureFile. `ConfigureFile` contains the
    /// target output path and the content required to create the file.
    ///
    /// # Arguments
    /// * `file` - The `ConfigureFile` describing where to write and what to write
    fn configure_file(&self, file: &ConfigureFile);

    /// Generates build instructions for installing header files
    ///
    /// This method should generate build steps to copy header files from
    /// their source locations to the appropriate installation directory,
    /// preserving directory structure as needed.
    ///
    /// # Arguments
    /// * `install_dir` - The base directory where headers should be installed
    /// * `headers` - List of header file paths to install
    fn install_headers(&self, install_dir: &Path, headers: &[Path]);
}
