use std::fmt::Display;
use std::path::PathBuf;

mod os;

use clap::{Parser, ValueEnum};
use os::Os;

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
    /// Cross compilation configuration file
    #[arg(long, value_name = "path")]
    cross_file: Vec<PathBuf>,

    /// Build type to use
    #[arg(long, value_name = "build type", default_value = "debug")]
    buildtype: BuildType,

    /// Installation prefix directory
    #[arg(long, value_name = "dir", default_value = "/usr/local")]
    prefix: PathBuf,

    /// Include directory (relative to prefix)
    #[arg(long)]
    includedir: Option<PathBuf>,

    /// Library directory (relative to prefix)
    #[arg(long)]
    libdir: Option<PathBuf>,

    /// Set project options (can be used multiple times)
    #[arg(short = 'D', value_name = "option=value")]
    define: Vec<String>,

    /// Build directory
    build_dir: PathBuf,

    /// Source directory (defaults to current directory if not specified)
    #[arg(default_value = ".")]
    source_dir: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let mut builder = picomeson::Meson::with_os(Os);

    // Add buildtype option
    builder.option("buildtype", args.buildtype.to_string());

    // Add prefix option
    builder.option("prefix", args.prefix.to_string_lossy());

    // Add includedir option if provided
    if let Some(includedir) = args.includedir {
        builder.option("includedir", includedir.to_string_lossy());
    }

    // Add libdir option if provided
    if let Some(libdir) = args.libdir {
        builder.option("libdir", libdir.to_string_lossy());
    }

    for define in args.define {
        if let Some((key, value)) = define.split_once('=') {
            builder.option(key, value);
            continue;
        }
        eprintln!("Ignoring unknown option: {define}");
    }

    builder.build(
        args.source_dir.to_string_lossy(),
        args.build_dir.to_string_lossy(),
    )?;

    Ok(())
}
