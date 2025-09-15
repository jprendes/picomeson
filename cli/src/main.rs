mod builder;
mod cli;
mod runtime;

use builder::Logging;
use runtime::Sandbox;

fn main() -> anyhow::Result<()> {
    let args = cli::parse();

    let mut builder = picomeson::Meson::new(Sandbox, Logging);

    // Add buildtype option
    builder.option("buildtype", args.buildtype.to_string());

    // Add prefix option
    builder.option("prefix", args.prefix.to_string_lossy());

    // Add user-defined options
    for d in args.define {
        builder.option(d.key, d.value);
    }

    builder.build(
        args.source_dir.to_string_lossy(),
        args.build_dir.to_string_lossy(),
    )?;

    Ok(())
}
