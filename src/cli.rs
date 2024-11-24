use clap::Parser;

#[derive(Debug, Parser)]
pub struct Args {
    #[arg(long, short = 'a', default_value = "./listing_37")]
    pub asm_bin_path: String,

    #[arg(long, short = 'o', default_value = "./output_recent.asm")]
    pub output_file: Option<String>,
}
