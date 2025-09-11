#![cfg_attr(not(test), no_std)]

extern crate alloc;

mod interpreter;
mod machine_file;
pub mod os;
mod parser;

use alloc::rc::Rc;
use alloc::string::String;

use hashbrown::HashMap;

use crate::os::Path;

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
        src_dir: impl AsRef<str>,
        build_dir: impl AsRef<str>,
    ) -> anyhow::Result<()> {
        let src_dir = Path::from(src_dir.as_ref());
        let build_dir = Path::from(build_dir.as_ref());

        let mut interp =
            interpreter::Interpreter::new(self.os.clone(), src_dir.clone(), build_dir)?;

        interp.interpret_string(include_str!("builtin-options.txt"))?;

        // prefix is a platform dependent option, so set it at runtime
        let default_prefix = self.os.default_prefix()?;
        interp.set_option("prefix", default_prefix.as_ref())?;

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
