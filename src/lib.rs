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
    build_dir: String,
    options: HashMap<String, String>,
}

impl Meson {
    pub fn new(os: impl os::Os) -> Self {
        let os = Rc::new(os);
        let build_dir = "build".into();
        let options = Default::default();
        Self {
            os,
            build_dir,
            options,
        }
    }

    pub fn option(&mut self, name: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.options.insert(name.into(), value.into());
        self
    }

    pub fn build(&self, src_dir: impl Into<String>) -> anyhow::Result<()> {
        let src_dir = src_dir.into();

        let mut interp = interpreter::Interpreter::new(self.os.clone(), &src_dir, &self.build_dir)?;

        interp.interpret_string(include_str!("builtin-options.txt"))?;

        let default_prefix = self.os.default_prefix()?;
        interp.interpret_string(&format!("option('prefix', type: 'string', value: '{default_prefix}', description: 'Installation prefix')"))?;

        let meson_options_path = self
            .os
            .join_paths(&[src_dir.as_str(), "meson_options.txt"])?;
        if self.os.exists(&meson_options_path).unwrap_or(false) {
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
