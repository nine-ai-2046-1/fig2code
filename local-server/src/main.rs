mod server;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about = "Start a simple local web server", long_about = None)]
struct Args {
    /// Server type: http
    server_type: String,
    /// Port number to bind
    port: u16,
    /// Mode: static (serve files)
    mode: String,
    /// Root folder to serve (relative to current dir or absolute)
    root: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Only allow localhost binding for security
    let bind_host = "127.0.0.1";

    match args.server_type.as_str() {
        "http" => {
            match args.mode.as_str() {
                "static" => {
                    let root = if args.root.is_absolute() {
                        args.root.clone()
                    } else {
                        std::env::current_dir()?.join(&args.root)
                    };

                    println!("Starting http static server on {}:{} serving {}", bind_host, args.port, root.display());
                    server::http::start_static_server(bind_host, args.port, root)?;
                }
                _ => {
                    eprintln!("Unsupported mode: {}", args.mode);
                    std::process::exit(2);
                }
            }
        }
        _ => {
            eprintln!("Unsupported server type: {}", args.server_type);
            std::process::exit(2);
        }
    }

    Ok(())
}
