use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Context;
use clap::{Parser, ValueEnum};

#[derive(ValueEnum, Clone, Debug)]
pub enum BuildType {
    /// Plain build type
    Plain,

    /// Debug build type
    Debug,

    /// Debug optimized build type
    DebugOptimized,

    /// Release build type
    Release,

    /// Min size build type
    MinSize,

    /// Custom build type
    Custom,
}

impl Display for BuildType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            BuildType::Plain => "plain",
            BuildType::Debug => "debug",
            BuildType::DebugOptimized => "debugoptimized",
            BuildType::Release => "release",
            BuildType::MinSize => "minsize",
            BuildType::Custom => "custom",
        };
        write!(f, "{s}")
    }
}

#[derive(Parser, Debug)]
#[command(name = "meson")]
#[command(about = "A minimal Meson build system implementation")]
#[command(version)]
pub struct Args {
    /// Build type to use
    #[arg(long, value_name = "build type", default_value = "debug")]
    pub buildtype: BuildType,

    /// Installation prefix directory
    #[arg(long, value_name = "dir", default_value = "/usr/local")]
    pub prefix: PathBuf,

    /// Set project options (can be used multiple times)
    #[arg(short = 'D', value_name = "option=value")]
    pub define: Vec<Define>,

    /// Build directory
    pub build_dir: PathBuf,

    /// Source directory (defaults to current directory if not specified)
    #[arg(default_value = ".")]
    pub source_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Define {
    pub key: String,
    pub value: String,
}

impl FromStr for Define {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (key, value) = s.split_once("=").context("No value specified for option")?;
        Ok(Define {
            key: key.to_string(),
            value: value.to_string(),
        })
    }
}

pub fn parse() -> Args {
    Args::parse()
}
