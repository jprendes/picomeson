#![no_std]

extern crate alloc;

mod interpreter;
pub mod os;
mod parser;

use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;

use hashbrown::HashMap;

pub struct Meson {
    os: Rc<dyn os::Os>,
    options: HashMap<String, String>,
}

impl Meson {
    pub fn with_os(os: impl os::Os) -> Self {
        let os = Rc::new(os);
        let options = Default::default();
        Self { os, options }
    }

    pub fn option(&mut self, name: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.options.insert(name.into(), value.into());
        self
    }

    pub fn build(
        &self,
        src_dir: impl Into<String>,
        build_dir: impl Into<String>,
    ) -> anyhow::Result<()> {
        let src_dir = src_dir.into();
        let build_dir = build_dir.into();

        let mut interp = interpreter::Interpreter::new(self.os.clone(), &src_dir, &build_dir)?;

        interp.interpret_string(include_str!("builtin-options.txt"))?;

        let default_prefix = self.os.default_prefix()?;
        interp.set_option("prefix", &default_prefix)?;

        let meson_options_path = self
            .os
            .join_paths(&[src_dir.as_str(), "meson_options.txt"])?;
        self.os
            .print(&format!("Looking for options file at {meson_options_path}"));
        if self.os.exists(&meson_options_path).unwrap_or(false) {
            self.os
                .print(&format!("Found options file at {meson_options_path}"));
            interp.interpret_file(&meson_options_path)?;
        }

        for (name, value) in &self.options {
            interp.set_option(name, value)?;
        }

        let meson_build_path = self.os.join_paths(&[src_dir.as_str(), "meson.build"])?;
        interp.interpret_file(&meson_build_path)?;

        Ok(())
    }
}
