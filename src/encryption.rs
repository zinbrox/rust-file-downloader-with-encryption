use chacha20poly1305::{
    aead::{stream, Aead, NewAead},
    XChaCha20Poly1305,
};
use rand::{rngs::OsRng, RngCore};
use anyhow::anyhow;
use std::{
    fs::{ self, File, OpenOptions },
    io::{ Read, Write },
    path::Path,
    error::Error
};
use serde_json::{json, Value};

pub fn cipher(option: &str, file_path: &str, destination_path: &str) -> Result<(), Box<dyn Error>>{
    let (mut key, mut nonce, mut large_key, mut large_nonce) = get_keys().unwrap();
    match option {
        "encryption" => {
            // encrypt_file(&key, &nonce, file_path, destination_path)?
            
            // TODO: add size check

            encrypt_large_file(&large_key, &large_nonce, file_path, destination_path)?
        },
        "decryption" => {
            decrypt_file(&key, &nonce, file_path, destination_path)?
        }
        _ => {
            println!("Invalid option")
        }
    }

    Ok(())
}

fn get_keys() -> Result<([u8; 32], [u8; 24], [u8; 32], [u8; 19]), Box<dyn Error>>{
    let file_path = "saved.json";
    let mut file;
    let (mut key, mut nonce) = ([0u8; 32], [0u8; 24]);
    let (mut large_key, mut large_nonce) = ([0u8; 32], [0u8; 19]);

    if Path::new(file_path).exists() {
        println!("File exists");
        file = OpenOptions::new().read(true).open(file_path);
        let mut contents = String::new();
        file?.read_to_string(&mut contents)?;
        println!("File contents: {}", contents);

        let json_value: Value = serde_json::from_str(&contents).unwrap();
        let key_str = json_value["key"].as_array().unwrap();
        let key_bytes: Vec<u8> = key_str.iter().map(|v| v.as_u64().unwrap() as u8).collect();
        
        let nonce_str = json_value["nonce"].as_array().unwrap();
        let nonce_bytes: Vec<u8> = nonce_str.iter().map(|v| v.as_u64().unwrap() as u8).collect();

        let large_key_str = json_value["large_key"].as_array().unwrap();
        let large_key_bytes: Vec<u8> = large_key_str.iter().map(|v| v.as_u64().unwrap() as u8).collect();

        let large_nonce_str = json_value["large_nonce"].as_array().unwrap();
        let large_nonce_bytes: Vec<u8> = large_nonce_str.iter().map(|v| v.as_u64().unwrap() as u8).collect();

        key = key_bytes.try_into().unwrap();
        nonce = nonce_bytes.try_into().unwrap();
        large_key = large_key_bytes.try_into().unwrap();
        large_nonce = large_nonce_bytes.try_into().unwrap();
    } else {
        println!("File does not exist");

        OsRng.fill_bytes(&mut key);
        OsRng.fill_bytes(&mut nonce);
        OsRng.fill_bytes(&mut large_key);
        OsRng.fill_bytes(&mut large_nonce);

        let content = json!({
            "key": key,
            "nonce": nonce,
            "large_key": large_key,
            "large_nonce": large_nonce
        });

        let mut file = OpenOptions::new()
                        .create(true)
                        .truncate(true)
                        .write(true)
                        .read(true)
                        .open(file_path)?;
        
        serde_json::to_writer(&mut file, &content)?;
    }

    // println!("key: {:?}, nonce: {:?}", key, nonce);
    
    Ok((key, nonce, large_key, large_nonce))
}

fn encrypt_file(key: &[u8; 32], nonce: &[u8; 24], file_path: &str, destination_path: &str) -> Result<(), anyhow::Error>{
    println!("Encrypting file");
    let cipher = XChaCha20Poly1305::new(key.into());

    let file_data = fs::read(file_path)?;

    let encrypted_data = cipher.encrypt(nonce.into(), file_data.as_ref()).map_err(|err| anyhow!("Encrypting file: {}", err))?;;

    fs::write(destination_path, encrypted_data)?;

    println!("File encrypted. You can find it at {}", destination_path);

    Ok(())
}

fn decrypt_file(key: &[u8; 32], nonce: &[u8; 24], file_path: &str, destination_path: &str) -> Result<(), anyhow::Error> {
    println!("Decrypting file");
    let cipher = XChaCha20Poly1305::new(key.into());

    let file_data = fs::read(file_path)?;

    let decrypted_data = cipher.decrypt(nonce.into(), file_data.as_ref()).map_err(|err| anyhow!("Decrypting file: {}", err))?;

    println!("Decrypted data: ");

    fs::write(destination_path, decrypted_data)?;

    println!("File decrypted. You can find it at {}", destination_path);

    Ok(())
}

fn encrypt_large_file(key: &[u8; 32], nonce: &[u8; 19], file_path: &str, destination_path: &str) -> Result<(), anyhow::Error> {
    println!("Encrypting large file");

    println!("key: {:?}, nonce: {:?}", key, nonce);

    let aead = XChaCha20Poly1305::new(key.as_ref().into());
    let mut stream_encryptor = stream::EncryptorBE32::from_aead(aead, nonce.as_ref().into());

    const BUFFER_LEN: usize = 500;
    let mut buffer = [0u8; BUFFER_LEN];

    let mut source_file = File::open(&file_path)?;
    let mut dist_file = File::create(destination_path)?;

    println!("before loop");

    loop {
        let read_count = source_file.read(&mut buffer)?;

        if read_count == BUFFER_LEN {
            let cipher_text = stream_encryptor
                                .encrypt_next(buffer.as_slice())
                                .map_err(|err| anyhow!("Encrypting file: {}", err))?;


            dist_file.write(&cipher_text)?;
        } else {
            let cipher_text = stream_encryptor
                                .encrypt_last(&buffer[..read_count])
                                .map_err(|err| anyhow!("Encrypting file: {}", err))?;

            dist_file.write(&cipher_text)?;
            break;
        }
    }

    Ok(())

}