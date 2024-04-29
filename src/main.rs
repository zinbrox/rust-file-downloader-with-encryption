use error_chain::error_chain;
// use std::io::copy;
use std::{io::{stdin, copy, Cursor}, path::Path};
use std::process::Command;
use serde_json;
use std::fs::OpenOptions;

mod downloader;
mod errorHandler;
mod encryption;

// error_chain !{
//     foreign_links {
//         Io(std::io::Error);
//         HttpRequest(reqwest::Error);
//         SerdeJson(serde_json::Error);
//     }
// }

use std::error::Error;

// api key = "AIzaSyDm9mTwoOzD6IW5ire-IcbvKJ3Ss9l0gDY"
// video url = https://www.youtube.com/watch?v=AfIOBLr1NDU

use regex::Regex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut option = String::new();
    println!("Enter an option\n1. Download Image\n2. Download Video\n3. Encrypt file\n4. Decrypt file");
    stdin().read_line(&mut option).expect("Failed to read option");

    match option.as_str().trim() {
        "1" => {
            // Downloand Image Stuff
            let mut file_name = String::new();
            println!("Enter the file name to download: ");
            stdin().read_line(&mut file_name).expect("Failed to read file name");

            let mut url = String::new();
            println!("Enter url: ");
            stdin().read_line(&mut url).expect("Failed to read url");

            download_and_create_image(&url, &file_name).await
        },
        "2" => {
            // Download Video stuff
            let mut video_url = String::new();
            println!("Enter the video url: ");
            stdin().read_line(&mut video_url).expect("Failed to read video url");

            let mut file_name = String::new();
            println!("Enter the name of the file: ");
            stdin().read_line(&mut file_name).expect("Failed to read file name");

            
            // let mut video_url = "https://www.youtube.com/watch?v=AfIOBLr1NDU";
            // let mut video_url = "https://www.youtube.com/watch?v=9bZkp7q19f0";
            // let mut file_name = "test";

            video_download(&(video_url.to_string()), &(file_name.to_string())).await
        },
        "3" => {
            let mut file_path = String::new();
            println!("Enter the file path: ");
            stdin().read_line(&mut file_path).expect("Failed to read file path");

            file_path = file_path.as_str().trim().to_string();

            let (file_path_without_ext, ext) = get_file_path_extracted(&file_path);

            let destination_path = &(file_path_without_ext.to_string() + "_encrypted" + "." + &ext);
            encryption::cipher("encryption", &file_path, destination_path)
        },
        "4" => {
            let mut file_path = String::new();
            println!("Enter the file path: ");
            stdin().read_line(&mut file_path).expect("Failed to read file path");

            file_path = file_path.as_str().trim().to_string();

            let (file_path_without_ext, ext) = get_file_path_extracted(&file_path);

            let destination_path = &(file_path_without_ext.to_string() + "_decrypted" + "." + &ext);
            encryption::cipher("decryption", &file_path, destination_path);
            Ok(())
        },
        _ => {
            println!("Invalid option");
            Ok(())
        }
    };

    Ok(())
}

fn get_file_path_extracted(file_path: &String) -> (String, String) {
    // Define a regular expression pattern to match the file extension
    let re = Regex::new(r"\.(\w+)$").unwrap();

    // Extract the extension using regex
    let ext = match re.captures(&file_path.as_str()) {
        Some(caps) => {
            caps.get(1).map_or("", |m| m.as_str())
        },
        None => "",
    };

    // Remove the extension from the file path
    let file_path_without_ext = re.replace(&file_path.as_str(), "").to_string();

    // Print the results
    println!("File path without extension: {}", file_path_without_ext);
    println!("Extension: {}", ext);

    (file_path_without_ext, ext.to_string())
}

async fn video_download(video_url: &String, file_name: &String) -> Result<(), Box<dyn Error>>{
    let folder_path = "videos";
    if !Path::new(folder_path).exists() {
        std::fs::create_dir_all(folder_path)?;
    }

    let mut final_path = folder_path.to_string() + "/" + file_name;
    final_path = final_path.trim_end_matches('\n').to_string();

    let quality = "1080p";
    let codec = "h264";
    let watermark = false;

    downloader::video_download(video_url, quality, codec, &final_path.as_str()).await;

    Ok(())
}


async fn download_and_create_image(url: &String, file_name: &String) -> Result<(), Box<dyn Error>> {
    // let url = "https://www.rust-lang.org/static/images/rust-logo-blk.svg";
    // let file_name = "rust-logo-blk.svg";
    let folder_path = "images";

    if !Path::new(folder_path).exists() {
        std::fs::create_dir_all(folder_path)?;
    }
    let response = reqwest::get(url).await?;
    
    let mut final_path = folder_path.to_string() + "/" + file_name;
    final_path = final_path.trim_end_matches('\n').to_string();

    if response.status().is_success() {
        let mut new_file = std::fs::File::create(final_path)?;
        let mut content = Cursor::new(response.bytes().await?);
        copy(&mut content, &mut new_file)?;
        println!("Success");
    } 

    Ok(())
}
