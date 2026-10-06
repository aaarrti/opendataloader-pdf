use clap::Parser;
use std::path::PathBuf;

use opendataloader_core::{convert_with_options, ConversionOptions};

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
    convert_with_options(
        &cli_args.input_paths,
        &cli_args.out_dir,
        ConversionOptions {
            json_enabled: cli_args.json,
            markdown_enabled: cli_args.markdown,
            image_output_enabled: cli_args.image,
            ..ConversionOptions::default()
        },
    )?;
    Ok(())
}
