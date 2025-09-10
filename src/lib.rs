#![no_std]

extern crate alloc;

mod interpreter;
pub mod os;
mod parser;

use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;

pub struct Meson<Os: os::Os> {
    os: Os,
    build_dir: String,
}

impl<Os: os::Os> Meson<Os> {
    pub fn new(os: Os) -> Self {
        let build_dir = "build".into();
        Self { os, build_dir }
    }

    pub fn build(self, src_dir: impl Into<String>) -> anyhow::Result<()> {
        let os = Rc::new(self.os);

        let src_dir = src_dir.into();

        let mut interpreter = interpreter::Interpreter::new(os.clone(), &src_dir, self.build_dir)?;
        interpreter.interpret_string(include_str!("builtin-options.txt"))?;

        let default_prefix = os.default_prefix()?;
        interpreter.interpret_string(&format!("option('prefix', type: 'string', value: '{default_prefix}', description: 'Installation prefix')"))?;

        let meson_options_path = os.join_paths(&[src_dir.as_str(), "meson_options.txt"])?;
        if os.exists(&meson_options_path).unwrap_or(false) {
            interpreter.interpret_file(&meson_options_path)?;
        }

        let meson_build_path = os.join_paths(&[src_dir.as_str(), "meson.build"])?;
        interpreter.interpret_file(&meson_build_path)?;

        Ok(())
    }
}
