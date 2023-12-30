#![feature(exit_status_error)]

use std::{borrow::BorrowMut, error::Error, path::Path, pin::Pin, sync::Arc};

use clap::{command, Parser};
use futures::{lock::Mutex, stream::FuturesUnordered, Future, StreamExt};
use tokio::{join, process::Command};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    no_png: bool,

    #[arg(long)]
    no_webp: bool,

    #[arg(long)]
    remove_larger_png: bool,

    #[arg(value_name = "FILE")]
    images: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Test file

    for p in &args.images {
        if p[p.len() - 4..].to_ascii_lowercase() != ".png" {
            panic!("Image is not PNG: {}", p)
        }
        if !Path::new(p).try_exists()? {
            panic!("Image does not exist: {}", p)
        }
    }

    // PNG conversion

    if !args.no_png {
        println!("Start PNG compression...");

        let queue_locked = Arc::new(Mutex::new(args.images.clone()));

        let cpu_count = num_cpus::get_physical();
        println!("CPU count: {}", cpu_count);

        let mut tasks = FuturesUnordered::<Pin<Box<dyn Future<Output = ()>>>>::new();
        for _ in 0..cpu_count {
            let qlc = queue_locked.clone();
            tasks.push(Box::pin(async move {
                loop {
                    let p: Option<String>;

                    {
                        let mut locked = qlc.lock().await;
                        let queue = locked.borrow_mut();
                        p = queue.pop();
                    }

                    if let Some(p) = p {
                        if let Err(err) = compress_png(&p).await {
                            println!("Failed to compress PNG: {}", err);
                        }
                    } else {
                        break;
                    }
                }
            }));
        }

        while let Some(()) = tasks.next().await {
            // No result
        }

        {
            let mut locked = queue_locked.lock().await;
            let queue = locked.borrow_mut();
            assert!(queue.is_empty());
        }
    }

    if !args.no_webp {
        // WebP conversion

        println!("Start WebP conversion...");

        for p in &args.images {
            if let Err(err) = compress_webp(p).await {
                println!("Failed to convert WebP: {}", err);
            }
        }

        // Check file size and remove larger one.

        for png_file in &args.images {
            let webp_file = format!("{}.webp", &png_file[..png_file.len() - 4]);

            let (png_size, webp_size): (Result<u64, std::io::Error>, Result<u64, std::io::Error>) = join!(
                async { Ok(tokio::fs::metadata(&png_file).await?.len()) },
                async { Ok(tokio::fs::metadata(&webp_file).await?.len()) }
            );

            if let (Ok(png_size), Ok(webp_size)) = (&png_size, &webp_size) {
                let file_to_be_removed: Option<&str>;

                if webp_size > png_size {
                    println!(
                        "`{}`: PNG {} < WebP {}, PNG win!",
                        &png_file, png_size, webp_size
                    );

                    file_to_be_removed = Some(&webp_file);
                } else {
                    println!(
                        "`{}`: PNG {} > WebP {}, WebP win!",
                        &png_file, png_size, webp_size
                    );

                    if args.remove_larger_png {
                        file_to_be_removed = Some(png_file);
                    } else {
                        file_to_be_removed = None;
                    }
                }

                if let Some(p) = file_to_be_removed {
                    if let Err(err) = tokio::fs::remove_file(&p).await {
                        println!("Failed to remove file `{}`: {}", &p, err);
                    }
                }
            } else {
                println!(
                    "Failed to get file size `{}`,{:?} , `{}`,{:?}",
                    png_file, png_size, webp_file, webp_size
                );
            }
        }
    }

    println!("Compression successful.");

    Ok(())
}

async fn compress_png(path_string: &str) -> Result<(), Box<dyn Error>> {
    Command::new("zopflipng")
        .args([
            "-m",
            "-y",
            "--keepchunks=cHRM,gAMA,hIST,iCCP,pHYs,sRGB",
            path_string,
            path_string,
        ])
        .status()
        .await?
        .exit_ok()?;

    Ok(())
}

async fn compress_webp(path_string: &str) -> Result<(), Box<dyn Error>> {
    Command::new("cwebp")
        .args([
            "-mt",
            "-z",
            "9",
            "-metadata",
            "icc",
            path_string,
            "-o",
            &format!("{}.webp", &path_string[..path_string.len() - 4]),
        ])
        .status()
        .await?
        .exit_ok()?;

    Ok(())
}
