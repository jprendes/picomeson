use picomeson::steps::{self, ConfigureFile};

pub struct Steps;

impl steps::BuildSteps for Steps {
    fn configure_file(&self, file: &ConfigureFile) {
        eprintln!(
            " > Configuring file {}: {} bytes",
            file.build_dir.join(&file.filename),
            file.content.len(),
        );
        if file.install {
            eprintln!(
                " > Installing header to {}: {} header",
                file.install_dir, file.filename
            );
        }
    }

    fn install_headers(
        &self,
        install_dir: &picomeson::path::Path,
        headers: &[picomeson::path::Path],
    ) {
        eprintln!(
            " > Installing headers to {install_dir}: {} headers",
            headers.len()
        );
    }

    fn build_executable(&self, target: &steps::BuildTarget) {
        eprintln!(
            " > Building executable {}: {} sources",
            target.install_dir.join(&target.filename),
            target.sources.len(),
        );
    }

    fn build_static_library(&self, target: &steps::BuildTarget) {
        let is_empty = target.sources.is_empty()
            || (target.sources.len() == 1 && target.sources[0].filename() == "empty.c");
        if target.install && !is_empty {
            eprintln!(
                " > Building static library {}: {} sources",
                target.install_dir.join(&target.filename),
                target.sources.len(),
            );
        }
    }
}
