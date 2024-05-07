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
    let (key, nonce, large_key, large_nonce) = get_keys().unwrap();
    let file_size = get_file_size_in_mb(file_path).unwrap();

    match option {
        "encryption" => {
            if file_size < 500.0 {
                encrypt_file(&key, &nonce, file_path, destination_path)?
            } else {
                encrypt_large_file(&large_key, &large_nonce, file_path, destination_path)?
            }
        },
        "decryption" => {
            if file_size < 500.0 {
                decrypt_file(&key, &nonce, file_path, destination_path)?
            } else {
                decrypt_large_file(&large_key, &large_nonce, file_path, destination_path)?
            }
        }
        _ => {
            println!("Invalid option")
        }
    }

    Ok(())
}

fn get_file_size_in_mb(file_path: &str) -> Result<f64, Box<dyn Error>> {
    let file = File::open(file_path)?;
    let file_size = file.metadata().unwrap().len();
    let mut size_in_mb = file_size as f64 / 1_048_576.0;
    size_in_mb = (size_in_mb * 100.0).round() / 100.0;

    Ok(size_in_mb)
}

fn extract_bytes(json_value: &Value, key: &str) -> Vec<u8> {
    json_value[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u8)
        .collect()
}

fn get_keys() -> Result<([u8; 32], [u8; 24], [u8; 32], [u8; 19]), Box<dyn Error>>{
    let file_path = "keys.json";
    let file;
    let (mut key, mut nonce) = ([0u8; 32], [0u8; 24]);
    let (mut large_key, mut large_nonce) = ([0u8; 32], [0u8; 19]);

    if Path::new(file_path).exists() {
        file = OpenOptions::new().read(true).open(file_path);
        let mut contents = String::new();
        file?.read_to_string(&mut contents)?;

        let json_value: Value = serde_json::from_str(&contents).unwrap();

        let key_bytes = extract_bytes(&json_value, "key");
        let nonce_bytes = extract_bytes(&json_value, "nonce");
        let large_key_bytes = extract_bytes(&json_value, "large_key");
        let large_nonce_bytes = extract_bytes(&json_value, "large_nonce");

        key = key_bytes.try_into().unwrap();
        nonce = nonce_bytes.try_into().unwrap();
        large_key = large_key_bytes.try_into().unwrap();
        large_nonce = large_nonce_bytes.try_into().unwrap();

        // println!("key: {:?}, nonce: {:?}, large_key: {:?}, large_nonce: {:?}", key, nonce, large_key, large_nonce);
    } else {
        // If file doesn't exist, generate new ones and store it there
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
    
    Ok((key, nonce, large_key, large_nonce))
}

fn encrypt_file(key: &[u8; 32], nonce: &[u8; 24], file_path: &str, destination_path: &str) -> Result<(), anyhow::Error>{
    println!("Encrypting file");
    let cipher = XChaCha20Poly1305::new(key.into());

    let file_data = fs::read(file_path)?;

    let encrypted_data = cipher.encrypt(nonce.into(), file_data.as_ref()).map_err(|err| anyhow!("Encrypting file: {}", err))?;

    fs::write(destination_path, encrypted_data)?;

    println!("File encrypted. You can find it at {}", destination_path);

    Ok(())
}

fn decrypt_file(key: &[u8; 32], nonce: &[u8; 24], file_path: &str, destination_path: &str) -> Result<(), anyhow::Error> {
    println!("Decrypting file");
    let cipher = XChaCha20Poly1305::new(key.into());

    let file_data = fs::read(file_path)?;

    let decrypted_data = cipher.decrypt(nonce.into(), file_data.as_ref()).map_err(|err| anyhow!("Decrypting file: {}", err))?;

    fs::write(destination_path, decrypted_data)?;

    println!("File decrypted. You can find it at {}", destination_path);

    Ok(())
}

fn encrypt_large_file(key: &[u8; 32], nonce: &[u8; 19], file_path: &str, destination_path: &str) -> Result<(), anyhow::Error> {
    let aead = XChaCha20Poly1305::new(key.as_ref().into());
    let mut stream_encryptor = stream::EncryptorBE32::from_aead(aead, nonce.as_ref().into());

    const BUFFER_LEN: usize = 500;
    let mut buffer = [0u8; BUFFER_LEN];

    let mut source_file = File::open(&file_path)?;
    let mut dist_file = File::create(destination_path)?;

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

    println!("File encrypted. You can find the file at {}", destination_path);

    Ok(())

}

fn decrypt_large_file(key: &[u8; 32], nonce: &[u8; 19], file_path: &str, destination_path: &str) -> Result<(), Box<dyn Error>> {
    let aead = XChaCha20Poly1305::new(key.as_ref().into());
    let mut stream_decryptor = stream::DecryptorBE32::from_aead(aead, nonce.as_ref().into());

    const BUFFER_LEN: usize = 500 + 16;
    let mut buffer = [0u8; BUFFER_LEN];

    let mut encrypted_file = File::open(file_path)?;
    let mut dist_file = File::create(destination_path)?;

    loop {
        let read_count = encrypted_file.read(&mut buffer)?;

        if read_count == BUFFER_LEN {
            let plain_text = stream_decryptor
                                .decrypt_next(buffer.as_slice())
                                .map_err(|err| anyhow!("Decrypting file: {}", err))?; 
            dist_file.write(&plain_text)?;
        } else if read_count == 0 {
            break;
        } else {
            let plain_text = stream_decryptor
                                .decrypt_last(&buffer[..read_count])
                                .map_err(|err| anyhow!("Decrypting file: {}", err))?;
            dist_file.write(&plain_text)?;
            break;
        }
    }

    println!("File decrypted. You can find the file at {}", destination_path);

    Ok(())
}