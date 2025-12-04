use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "rcss")]
#[command(about = "Rusty Style Sheets compiler")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Parser, Debug)]
pub enum Commands {
    Build {
        input: String,
        #[arg(short, long)]
        output: Option<String>,
    },
}
