use clap::Parser;
use std::path::PathBuf;

use opendataloader_core as odl;

#[derive(Debug, Parser)]
struct CliArg {
    #[arg(short, long, required = true)]
    input_paths: Vec<PathBuf>,
    #[arg(short, long, required = true)]
    out_dir: PathBuf,
    #[arg(short, long, required = false)]
    json: bool,
    #[arg(short, long, required = false)]
    markdown: bool,
    #[arg(short, long, required = false)]
    image: bool,
}

fn main() -> anyhow::Result<()> {
    let cli_args = CliArg::try_parse()?;
    odl::convert(
        cli_args.input_paths,
        cli_args.out_dir,
        cli_args.json,
        cli_args.markdown,
        cli_args.image,
    )?;
    Ok(())
}
