use alloc::string::String;
use core::fmt;

#[derive(Clone, PartialEq, Eq, Hash, Default)]
pub struct Path(String);

const SEP: &str = "/";

impl AsRef<str> for Path {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Path({})", self.0)
    }
}

impl Path {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from(path: impl AsRef<str>) -> Self {
        Self(path.as_ref().replace("\\", "/").into())
    }

    pub fn set_extension(&self, suffix: &str) -> Self {
        let path = &self.0;
        // Find the last occurrence of '/'
        let last_separator = path.rfind(SEP);

        // Find the last occurrence of '.' after the last separator (or from start if no separator)
        let search_start = last_separator.map(|i| i + 1).unwrap_or(0);
        let last_dot = path[search_start..].rfind('.').map(|i| search_start + i);

        if let Some(dot_pos) = last_dot {
            // Replace everything after the last dot with the new suffix
            let mut new_path = String::from(&path[..dot_pos]);
            if !suffix.starts_with('.') {
                new_path.push('.');
            }
            new_path.push_str(suffix);
            Self(new_path)
        } else {
            // No extension found, append the suffix
            let mut new_path = String::from(path);
            if !suffix.starts_with('.') {
                new_path.push('.');
            }
            new_path.push_str(suffix);
            Self(new_path)
        }
    }

    pub fn join(&self, path: impl AsRef<str>) -> Self {
        if path.as_ref().starts_with(SEP) || self.0.is_empty() {
            return Self(path.as_ref().into());
        }

        let mut new_path = String::from(self.0.trim_end_matches(SEP));
        new_path.push_str(SEP);
        new_path.push_str(path.as_ref());
        Self(new_path)
    }
}
