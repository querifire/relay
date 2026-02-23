//! Encrypt/decrypt instance passwords at rest. Key is stored in config dir.
//! On Windows the key file is protected with DPAPI (user-only decryption).

use anyhow::{anyhow, Result};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::ChaCha20Poly1305;
use std::path::PathBuf;

const KEY_FILE: &str = "relay/.credkey";
const NONCE_LEN: usize = 12;
const DPAPI_MAGIC: &[u8] = b"DPAPI";

#[cfg(windows)]
fn protect_key_win(key: &[u8; 32]) -> Result<Vec<u8>> {
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN,
    };

    unsafe {
        let mut data_in = CRYPT_INTEGER_BLOB {
            cbData: 32,
            pbData: key.as_ptr() as *mut u8,
        };
        let mut data_out = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: null_mut(),
        };
        if CryptProtectData(
            &mut data_in,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut data_out,
        ) == 0
        {
            return Err(anyhow!("DPAPI CryptProtectData failed"));
        }
        let out = std::slice::from_raw_parts(data_out.pbData, data_out.cbData as usize).to_vec();
        let _ = LocalFree(data_out.pbData as _);
        Ok(out)
    }
}

#[cfg(windows)]
fn unprotect_key_win(encrypted: &[u8]) -> Result<[u8; 32]> {
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN,
    };

    let mut key = [0u8; 32];
    unsafe {
        let mut data_in = CRYPT_INTEGER_BLOB {
            cbData: encrypted.len() as u32,
            pbData: encrypted.as_ptr() as *mut u8,
        };
        let mut data_out = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: null_mut(),
        };
        if CryptUnprotectData(
            &mut data_in,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut data_out,
        ) == 0
        {
            return Err(anyhow!("DPAPI CryptUnprotectData failed"));
        }
        if data_out.cbData >= 32 {
            key.copy_from_slice(std::slice::from_raw_parts(data_out.pbData, 32));
        }
        let _ = LocalFree(data_out.pbData as _);
    }
    Ok(key)
}

fn key_path() -> Result<PathBuf> {
    let base = dirs::config_dir().ok_or_else(|| anyhow!("No config dir"))?;
    Ok(base.join(KEY_FILE))
}

fn get_or_create_key() -> Result<[u8; 32]> {
    let path = key_path()?;
    if path.exists() {
        let bytes = std::fs::read(&path)?;
        let mut key = [0u8; 32];
        #[cfg(windows)]
        if bytes.starts_with(DPAPI_MAGIC) && bytes.len() > DPAPI_MAGIC.len() {
            return unprotect_key_win(&bytes[DPAPI_MAGIC.len()..]).map_err(|e| anyhow!("{}", e));
        }
        if bytes.len() >= 32 {
            key.copy_from_slice(&bytes[..32]);
            return Ok(key);
        }
    }
    let key: [u8; 32] = rand::random();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    #[cfg(windows)]
    {
        let protected = protect_key_win(&key)?;
        let mut out = Vec::with_capacity(DPAPI_MAGIC.len() + protected.len());
        out.extend_from_slice(DPAPI_MAGIC);
        out.extend_from_slice(&protected);
        std::fs::write(&path, &out)?;
    }
    #[cfg(not(windows))]
    {
        std::fs::write(&path, &key)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&path)?.permissions();
            perms.set_mode(0o600);
            let _ = std::fs::set_permissions(&path, perms);
        }
    }
    Ok(key)
}

/// Encrypt a password for storage. Returns base64-encoded ciphertext (nonce + ciphertext).
pub fn encrypt_password(plain: &str) -> Result<String> {
    let key = get_or_create_key()?;
    let cipher = ChaCha20Poly1305::new_from_slice(&key).map_err(|e| anyhow!("{:?}", e))?;
    let nonce: [u8; NONCE_LEN] = rand::random();
    let ciphertext = cipher
        .encrypt((&nonce).into(), plain.as_bytes())
        .map_err(|e| anyhow!("encrypt: {:?}", e))?;
    let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &out,
    ))
}

/// Decrypt a password from storage. Input is base64-encoded (nonce + ciphertext).
pub fn decrypt_password(encoded: &str) -> Result<String> {
    let key = get_or_create_key()?;
    let bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        encoded.trim(),
    )
    .map_err(|e| anyhow!("base64: {}", e))?;
    if bytes.len() < NONCE_LEN {
        return Err(anyhow!("Invalid encrypted payload"));
    }
    let cipher = ChaCha20Poly1305::new_from_slice(&key).map_err(|e| anyhow!("{:?}", e))?;
    let (n, ct) = bytes.split_at(NONCE_LEN);
    let mut nonce_arr = [0u8; NONCE_LEN];
    nonce_arr.copy_from_slice(n);
    let plain = cipher
        .decrypt((&nonce_arr).into(), ct)
        .map_err(|_| anyhow!("Decryption failed (wrong key or corrupted)"))?;
    String::from_utf8(plain).map_err(|e| anyhow!("UTF-8: {}", e))
}
