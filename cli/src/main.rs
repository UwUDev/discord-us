use clap::{Parser, Args, Subcommand};
use clap::builder::Str;

use discord_us::common::Waterfall;

#[derive(Parser, Debug)]
#[command(name = "discord-us", version = "0.1.0", about = "Discord Unlimited Storage")]
struct Cli {
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand, Debug)]
enum Commands {
    Download {
        #[arg(short, long)]
        password: Option<String>,

        #[arg(short, long)]
        waterfall: String,

        #[arg(short, long)]
        output: String
    }
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Download { password, waterfall, output } => {
            println!("Downloading waterfall {} to {}", waterfall, output);

            let waterfall = Waterfall::from_file(waterfall);
        }
    }
}
