mod utils;

use clap::{Parser, Args, Subcommand};
use clap::builder::Str;

use discord_us::common::{Waterfall, FileReadable, Subscription, FileWritable};
use discord_us::downloader::{FileDownloader, Downloader, WaterfallDownloader};

use std::time::Instant;

use bytesize::ByteSize;
use discord_us::uploader::{FileUploader, Uploader, WaterfallExporter};

#[derive(Parser, Debug)]
#[command(name = "discord-us", version = "0.1.0", about = "Discord Unlimited Storage")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Download {
        #[arg(short, long)]
        password: Option<String>,

        #[arg(short, long)]
        waterfall: String,

        #[arg(short, long)]
        output: String,
    },

    Upload {
        #[arg(short, long)]
        password: Option<String>,

        #[arg(short, long)]
        waterfall: String,

        #[arg(short, long)]
        input: String,

        #[arg(short, long, default_value_t = Subscription::Free.get_max_chunk_upload_size())]
        container_size: usize,

        #[arg(short, long)]
        token: String,

        #[arg(long)]
        channel_id: u64,
    },
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Download { password, waterfall, output } => {
            let waterfall = Waterfall::from_file(waterfall);

            println!("Downloading file {} ({}) into {}", waterfall.filename, ByteSize(waterfall.size).to_string_as(true), output);

            let mut file_downloader = FileDownloader::from_waterfall(waterfall.clone());
            let now = Instant::now();

            if let Some(password) = password {
                file_downloader.set_password(password);
            }

            file_downloader.download_file(output);

            println!("Downloaded succeed {:?}", now.elapsed());
        }
        Commands::Upload { input, password, waterfall, container_size, channel_id, token } => {
            let mut file_uploader = FileUploader::new(input, container_size as u32);
            let now = Instant::now();

            let pass = match password.clone() {
                None => utils::create_random_password(16),
                Some(pass) => pass
            };

            file_uploader.upload(pass.clone(), token, channel_id);

            let waterfall_struct = if password.is_some() {
                file_uploader.export_waterfall()
            } else {
                file_uploader.export_waterfall_with_password(pass.clone())
            };

            println!("Exporting waterfall");

            waterfall_struct.write_to_file(waterfall.clone());

            println!("Uploaded succeed {:?}", now.elapsed());
        }
    };
}