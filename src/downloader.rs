use reqwest;
use std::collections::HashMap;
use serde_json::{self, Value};
use std::error::Error;
use std::io::Cursor;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{Write};
use std::fs::File;
use std::fs::remove_file;
use std::cmp::min;

use futures_util::{StreamExt, TryStreamExt};
use bytes::BytesMut;

use std::io::stdin;

use crate::errorHandler::end_program_error;
use crate::encryption::cipher;

pub async fn video_download(url: &str, quality: &str, codec: &str, file_path: &str) {
    println!("Fetching details for url {}", url);

    let mut body = HashMap::new();
    body.insert("url", url);
    body.insert("vQuality", quality);
    body.insert("vCodec", codec);

    let apiUrl = "co.wuk.sh";
    let stream_url = format!("https://{apiUrl}/api/json");

    getVideoStream(&stream_url, &body, &file_path).await;
}

async fn getVideoStream(url: &str, body: &HashMap<&str, &str>, file_path: &str) {
    println!("in getVideoStream");
    let client = reqwest::Client::new();

    let response = client.post(url)
        .header("content_type", "application/json") // change to small
        .header("accept", "application/json")
        .json(&body)
        .send()
        .await;
    
    let formatted_response = response.expect("should be a response").text().await.unwrap();

    // println!("{:?}", formatted_response);

    let deserialised_response: Value = serde_json::from_str(&formatted_response).unwrap();
    // println!("Deserialised response: {:?}", deserialised_response);

    if deserialised_response.get("status").unwrap() == "stream" {
        let stream_url = deserialised_response.get("url").unwrap().to_string();
        let cleaned_url = &stream_url[1..stream_url.len() - 1];

        downVideoFromStream(&cleaned_url.to_string(), file_path).await;
    } else {
        end_program_error("Failed to get stream url");
    }
}

async fn downVideoFromStream(stream_url: &String, file_path: &str) -> Result<(), Box<dyn Error>>{
    println!("Found stream url, starting download");
    
    let client = reqwest::Client::new();
    let res = client.get(stream_url.to_string())
                    .send()
                    .await?;
    
    // println!("Response: {:?}", res);

    // Get the video extension
    let video_type = res.headers()
                        .get("content-type")
                        .unwrap()
                        .to_str()
                        .unwrap();

    let mut video_extension = match video_type.find('/') {
        Some(index) => &video_type[index + 1..],
        None => "",
    };

    let full_file_path = &(file_path.to_string() + "." + video_extension);
    let encrypted_file_path = &(file_path.to_string() + "_encrypted." + video_extension);
    // println!("file path: {}", full_file_path);
    // println!("headers: {}", video_extension);

    let total_size = res
        .content_length()
        .ok_or(format!("Failed to get content length from '{}'", stream_url))?;

    let size_in_mb = total_size as f64 / 1_048_576.0;
    println!("Total size: {} MB. Do you want to start download?", format!("{:.2}", size_in_mb));
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

    println!("Do you want to encrypt the file? (yes/no)");
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