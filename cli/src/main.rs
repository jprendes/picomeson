use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;

mod os;
mod steps;

use anyhow::Context;
use clap::{Parser, ValueEnum};
use os::Os;
use steps::Steps;

#[derive(ValueEnum, Clone, Debug)]
enum BuildType {
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
struct Args {
    /// Build type to use
    #[arg(long, value_name = "build type", default_value = "debug")]
    buildtype: BuildType,

    /// Installation prefix directory
    #[arg(long, value_name = "dir", default_value = "/usr/local")]
    prefix: PathBuf,

    /// Set project options (can be used multiple times)
    #[arg(short = 'D', value_name = "option=value")]
    define: Vec<Define>,

    /// Build directory
    build_dir: PathBuf,

    /// Source directory (defaults to current directory if not specified)
    #[arg(default_value = ".")]
    source_dir: PathBuf,
}

#[derive(Debug, Clone)]
struct Define {
    key: String,
    value: String,
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

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let mut builder = picomeson::Meson::new(Os, Steps);

    // Add buildtype option
    builder.option("buildtype", args.buildtype.to_string());

    // Add prefix option
    builder.option("prefix", args.prefix.to_string_lossy());

    // Add user-defined options
    for Define { key, value } in args.define {
        builder.option(key, value);
    }

    builder.build(
        args.source_dir.to_string_lossy(),
        args.build_dir.to_string_lossy(),
    )?;

    Ok(())
}
