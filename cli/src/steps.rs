use picomeson::steps;

pub struct Steps;

impl steps::BuildSteps for Steps {
    fn write_file(&self, path: &picomeson::path::Path, content: &str) {
        eprintln!("Writing file {} bytes to {path}", content.len());
    }

    fn install_headers(
        &self,
        install_dir: &picomeson::path::Path,
        headers: &[picomeson::path::Path],
    ) {
        eprintln!("Installing {} headers to {install_dir}", headers.len());
    }

    fn build_executable(&self, target: &steps::BuildTarget) {
        eprintln!("Building executable: {}", target.name);
    }

    fn build_static_library(&self, target: &steps::BuildTarget) {
        eprintln!(
            "Building static library: {}{}",
            target.name,
            if target.install {
                ""
            } else {
                " (not installed)"
            }
        );
    }
}
