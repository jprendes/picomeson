#![cfg_attr(not(test), no_std)]

extern crate alloc;

mod interpreter;
mod machine_file;
pub mod os;
mod parser;
pub mod path;
pub mod steps;

use alloc::rc::Rc;
use alloc::string::String;

use hashbrown::HashMap;

use crate::path::Path;

pub struct Meson {
    os: Rc<dyn os::Os>,
    steps: Rc<dyn steps::BuildSteps>,
    options: HashMap<String, String>,
}

impl Meson {
    pub fn new(os: impl os::Os, steps: impl steps::BuildSteps) -> Self {
        let os = Rc::new(os);
        let steps = Rc::new(steps);
        let options = Default::default();
        Self { os, steps, options }
    }

    pub fn option(&mut self, name: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.options.insert(name.into(), value.into());
        self
    }

    pub fn build(
        &self,
        src_dir: impl AsRef<str>,
        build_dir: impl AsRef<str>,
    ) -> anyhow::Result<()> {
        let src_dir = Path::from(src_dir.as_ref());
        let build_dir = Path::from(build_dir.as_ref());

        let mut interp = interpreter::Interpreter::new(
            self.os.clone(),
            self.steps.clone(),
            src_dir.clone(),
            build_dir,
        )?;

        interp.interpret_string(include_str!("builtin-options.txt"))?;

        let meson_options_path = src_dir.join("meson_options.txt");
        if self.os.exists(&meson_options_path).unwrap_or(false) {
            interp.interpret_file(&meson_options_path)?;
        }

        for (name, value) in &self.options {
            interp.set_option(name, value)?;
        }

        let meson_build_path = src_dir.join("meson.build");
        interp.interpret_file(&meson_build_path)?;

        Ok(())
    }
}
