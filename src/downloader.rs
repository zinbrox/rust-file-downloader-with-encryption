use reqwest;
use std::collections::HashMap;
use serde_json::{self, Value};
use std::error::Error;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{Write};
use std::fs::File;
use std::fs::remove_file;
use std::cmp::min;
use futures_util::{StreamExt};
use std::io::stdin;

use crate::error_handler::end_program_error;
use crate::encryption::cipher;

async fn download_and_create_image(url: &String, file_name: &String) -> Result<(), Box<dyn Error>> {
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
        println!("File downloaded!");
    } 

    Ok(())
}

pub async fn video_download(url: &str, quality: &str, codec: &str, file_path: &str) {
    println!("Fetching details for url {}", url);

    let mut body = HashMap::new();
    body.insert("url", url);
    body.insert("vQuality", quality);
    body.insert("vCodec", codec);

    let api_url = "co.wuk.sh";
    let stream_url = format!("https://{api_url}/api/json");

    get_video_stream(&stream_url, &body, &file_path).await;
}

async fn get_video_stream(url: &str, body: &HashMap<&str, &str>, file_path: &str) {
    let client = reqwest::Client::new();

    let response = client.post(url)
        .header("content_type", "application/json") // change to small
        .header("accept", "application/json")
        .json(&body)
        .send()
        .await;
    
    let formatted_response = response.expect("should be a response").text().await.unwrap();

    let deserialised_response: Value = serde_json::from_str(&formatted_response).unwrap();

    if deserialised_response.get("status").unwrap() == "stream" {
        let stream_url = deserialised_response.get("url").unwrap().to_string();
        let cleaned_url = &stream_url[1..stream_url.len() - 1];

        download_video_from_stream(&cleaned_url.to_string(), file_path).await;
    } else {
        end_program_error("Failed to get stream url");
    }
}

async fn download_video_from_stream(stream_url: &String, file_path: &str) -> Result<(), Box<dyn Error>>{
    println!("Found stream url, fetching video details...");
    
    let client = reqwest::Client::new();
    let res = client.get(stream_url.to_string())
                    .send()
                    .await?;

    // Get the video extension
    let video_type = res.headers()
                        .get("content-type")
                        .unwrap()
                        .to_str()
                        .unwrap();

    let video_extension = match video_type.find('/') {
        Some(index) => &video_type[index + 1..],
        None => "",
    };

    let full_file_path = &(file_path.to_string() + "." + video_extension);
    let encrypted_file_path = &(file_path.to_string() + "_encrypted." + video_extension);

    let total_size = res
        .content_length()
        .ok_or(format!("Failed to get content length from '{}'", stream_url))?;

    let size_in_mb = total_size as f64 / 1_048_576.0;
    println!("Total size: {} MB. Do you want to start download? (y/n)", format!("{:.2}", size_in_mb));
    let mut continue_option = String::new();
    stdin().read_line(&mut continue_option).expect("Failed to read use response");

    match continue_option.trim().to_lowercase().as_str() {
        "yes" | "y" | "" => { println!("Continuing..."); }, // this won't be printed
        _ => { end_program_error("User aborted download!") }
    }


    // Indicatif progress bar setup
    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
    );
    pb.set_message(format!("Downloading {}", stream_url));

    // download in chunks
    let mut file = File::create(full_file_path).or(Err(format!("Failed to create file {}", full_file_path)))?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.or(Err(format!("Error while downloading file")))?;
        file.write(&chunk)
            .or(Err(format!("Error while writing to file")))?;
        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }

    pb.finish_with_message(format!("Downloading video to {}", full_file_path));

    println!("Downloaded file to {}", full_file_path);

    println!("Do you want to encrypt the file? (y/n)");
    let mut encrypt_option = String::new();
    stdin().read_line(&mut encrypt_option).expect("Failed to read use response");

    match encrypt_option.trim().to_lowercase().as_str() {
        "yes" | "y" | "" => { 
            println!("Continuing..."); 
            cipher("encryption", full_file_path, encrypted_file_path);
            remove_file(full_file_path).expect("Failed to remove file");
        },
        _ => { 
            println!("Cancelled file encryption!");
        }
    }

    println!("Adios");

    Ok(())
}