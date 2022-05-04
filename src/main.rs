use clap::Parser;
use m3u8::m3u8_download::M3U8Download;

/// Download M3U8 Video
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Download video index address
    #[clap(short, long)]
    url: String,

    /// Save video directory
    #[clap(short, long, default_value = "./output")]
    output: String,

    /// Thread of number
    #[clap(short, long, default_value_t = 3)]
    thread: u8,
}


#[tokio::main]
async fn main() {
    let args: Args = Args::parse();
    if args.url.is_empty() || args.output.is_empty() {
        return;
    }
    println!("Download: {}", args.url);
    println!("Out: {}", args.output);
    let download = M3U8Download::from(args.url, args.output, args.thread);
    match download.start().await {
        Ok(_) => println!("Download Success !"),
        Err(err) => println!("{:?}", err)
    };
}