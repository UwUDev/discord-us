mod utils;

use std::thread::sleep;
use std::io::{stdout, Write};
use clap::{Parser, Subcommand};
// use clap::builder::Str;

use discord_us::common::{Waterfall, FileReadable, Subscription, FileWritable};
use discord_us::downloader::{FileDownloader, Downloader, WaterfallDownloader};
use discord_us::signal::{PartProgression, Signal};

use std::time::Instant;

use bytesize::ByteSize;
use discord_us::uploader::{FileUploadArguments, FileUploader, Uploader, WaterfallExporter};
use crate::utils::to_progress_bar;

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
            let mut signal: PartProgression<u64> = PartProgression::new();

            let mut file_uploader = FileUploader::new(input, container_size as u32);
            let now = Instant::now();

            let pass = match password.clone() {
                None => utils::create_random_password(16),
                Some(pass) => pass
            };

            let mut upload_args = FileUploadArguments::new(pass.clone(), token.clone(), channel_id);

            upload_args.with_signal(&signal);

            let total_upload_size = file_uploader.upload(upload_args);

            let start = Instant::now();

            println!("\n");

            loop {
                sleep(std::time::Duration::from_millis(50));

                signal.retrim_ranges();

                let progress = signal.get_total();
                let data = signal.get_data();

                let elapsed = start.elapsed().as_secs_f64();

                let bar = to_progress_bar(data, total_upload_size, 50, '#', '-');

                print!("\rProgress: {} {}/{} ({}/s) ({:.2}%)",
                         bar,
                         ByteSize(progress).to_string_as(true),
                         ByteSize(total_upload_size).to_string_as(true),
                         ByteSize((progress as f64 / elapsed) as u64).to_string_as(true),
                         (progress as f64 / total_upload_size as f64) * 100.0);

                stdout().flush().unwrap();

                if progress == total_upload_size {
                    println!("\nFinalizing upload...");
                    sleep(std::time::Duration::from_millis(5000));
                    break;
                }
            }

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