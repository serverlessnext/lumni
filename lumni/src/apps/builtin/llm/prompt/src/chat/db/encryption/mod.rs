use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use base64::engine::general_purpose;
use base64::Engine;
use lumni::api::error::{ApplicationError, EncryptionError};
use ring::aead;
use ring::rand::{SecureRandom, SystemRandom};
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::pkcs1v15::Pkcs1v15Encrypt;
use rsa::pkcs8::{
    DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey,
    LineEnding,
};
use rsa::{BigUint, RsaPrivateKey, RsaPublicKey};
use sha2::{Digest, Sha256};

use crate::external as lumni;

#[derive(Debug)]
pub struct EncryptionHandler {
    public_key: RsaPublicKey,
    private_key: RsaPrivateKey,
    key_path: PathBuf,
    sha256_hash: String,
}

impl EncryptionHandler {
    pub fn new(
        public_key_pem: &str,
        private_key_pem: &str,
        key_path: PathBuf,
    ) -> Result<Self, ApplicationError> {
        let public_key = RsaPublicKey::from_public_key_pem(public_key_pem)
            .map_err(EncryptionError::from)?;
        let private_key = RsaPrivateKey::from_pkcs8_pem(private_key_pem)
            .map_err(EncryptionError::from)?;

        let sha256_hash = Self::sha256_hash(private_key_pem);

        Ok(Self {
            public_key,
            private_key,
            key_path,
            sha256_hash,
        })
    }

    pub fn new_from_path(
        private_key_path: &PathBuf,
    ) -> Result<Option<Self>, ApplicationError> {
        if !private_key_path.exists() {
            return Err(ApplicationError::NotFound(format!(
                "Private key file not found: {:?}",
                private_key_path
            )));
        }

        let private_key_pem = fs::read_to_string(private_key_path)
            .map_err(|e| ApplicationError::IOError(e))?;

        let private_key = if Self::is_encrypted_key(&private_key_pem) {
            Self::parse_encrypted_private_key(
                private_key_path.to_str().unwrap(),
                &private_key_pem,
            )?
        } else {
            Self::parse_private_key(private_key_path.to_str().ok_or_else(
                || ApplicationError::InvalidInput("Invalid path".to_string()),
            )?)?
        };

        let public_key = RsaPublicKey::from(&private_key);
        let public_key_pem = public_key
            .to_public_key_pem(LineEnding::LF)
            .map_err(|e| EncryptionError::Other(Box::new(e)))?;
        let private_key_pem = private_key
            .to_pkcs8_pem(LineEnding::LF)
            .map_err(|e| EncryptionError::Pkcs8Error(e))?;

        Ok(Some(Self::new(
            &public_key_pem,
            &private_key_pem,
            private_key_path.clone(),
        )?))
    }

    pub fn get_private_key_pem(&self) -> Result<String, ApplicationError> {
        self.private_key
            .to_pkcs8_pem(LineEnding::LF)
            .map(|pem| pem.to_string())
            .map_err(|e| {
                ApplicationError::EncryptionError(EncryptionError::Pkcs8Error(
                    e.into(),
                ))
            })
    }

    pub fn get_key_path(&self) -> &PathBuf {
        &self.key_path
    }

    pub fn get_sha256_hash(&self) -> &str {
        &self.sha256_hash
    }

    fn is_encrypted_key(key_pem: &str) -> bool {
        key_pem.contains("ENCRYPTED")
    }

    fn parse_encrypted_private_key(
        path: &str,
        key_pem: &str,
    ) -> Result<RsaPrivateKey, ApplicationError> {
        println!("The private key at {:?} is password-protected.", path);

        let mut password = String::new();
        print!("Enter the password for the private key: ");
        io::stdout()
            .flush()
            .map_err(|e| ApplicationError::IOError(e))?;
        io::stdin()
            .read_line(&mut password)
            .map_err(|e| ApplicationError::IOError(e))?;
        let password = password.trim();

        RsaPrivateKey::from_pkcs8_encrypted_pem(key_pem, password).map_err(
            |e| {
                ApplicationError::EncryptionError(EncryptionError::RsaError(
                    rsa::Error::Pkcs8(e),
                ))
            },
        )
    }

    fn parse_private_key(
        key_path: &str,
    ) -> Result<RsaPrivateKey, ApplicationError> {
        let key_data = fs::read_to_string(key_path)
            .map_err(|e| ApplicationError::IOError(e))?;

        // Try parsing as OpenSSH format
        if key_data.starts_with("-----BEGIN OPENSSH PRIVATE KEY-----") {
            return Self::parse_openssh_private_key(&key_data);
        }

        // Try parsing as PKCS#8 PEM
        if let Ok(key) = RsaPrivateKey::from_pkcs8_pem(&key_data) {
            return Ok(key);
        }

        // Try parsing as PKCS#1 PEM
        if let Ok(key) = RsaPrivateKey::from_pkcs1_pem(&key_data) {
            return Ok(key);
        }

        // If all parsing attempts fail, return an error
        Err(ApplicationError::InvalidInput(
            "Unable to parse private key: unsupported format".to_string(),
        ))
    }

    fn parse_openssh_private_key(
        key_data: &str,
    ) -> Result<RsaPrivateKey, ApplicationError> {
        let lines: Vec<&str> = key_data.lines().collect();

        if !lines[0].starts_with("-----BEGIN OPENSSH PRIVATE KEY-----") {
            return Err(ApplicationError::InvalidInput(
                "Not an OpenSSH private key".to_string(),
            ));
        }

        let base64_data = lines[1..lines.len() - 1].join("");
        let decoded =
            general_purpose::STANDARD.decode(base64_data).map_err(|e| {
                ApplicationError::from(EncryptionError::Base64Error(e))
            })?;

        // OpenSSH magic header
        if &decoded[0..15] != b"openssh-key-v1\0" {
            return Err(ApplicationError::InvalidInput(
                "Invalid OpenSSH key format".to_string(),
            ));
        }

        let mut index = 15;

        // Skip ciphername, kdfname, kdfoptions
        for _ in 0..3 {
            let len = u32::from_be_bytes([
                decoded[index],
                decoded[index + 1],
                decoded[index + 2],
                decoded[index + 3],
            ]) as usize;
            index += 4 + len;
        }

        // Number of keys (should be 1)
        index += 4;

        // Public key length
        let pubkey_len = u32::from_be_bytes([
            decoded[index],
            decoded[index + 1],
            decoded[index + 2],
            decoded[index + 3],
        ]) as usize;
        index += 4;

        // Skip public key
        index += pubkey_len;

        // Private key length
        let privkey_len = u32::from_be_bytes([
            decoded[index],
            decoded[index + 1],
            decoded[index + 2],
            decoded[index + 3],
        ]) as usize;
        index += 4;

        // Skip checkints
        index += 8;

        // Key type length
        let key_type_len = u32::from_be_bytes([
            decoded[index],
            decoded[index + 1],
            decoded[index + 2],
            decoded[index + 3],
        ]) as usize;
        index += 4;

        // Skip key type
        index += key_type_len;

        // Extract n
        let n_len = u32::from_be_bytes([
            decoded[index],
            decoded[index + 1],
            decoded[index + 2],
            decoded[index + 3],
        ]) as usize;
        index += 4;
        let n = BigUint::from_bytes_be(&decoded[index..index + n_len]);
        index += n_len;

        // Extract e
        let e_len = u32::from_be_bytes([
            decoded[index],
            decoded[index + 1],
            decoded[index + 2],
            decoded[index + 3],
        ]) as usize;
        index += 4;
        let e = BigUint::from_bytes_be(&decoded[index..index + e_len]);
        index += e_len;

        // Extract d
        let d_len = u32::from_be_bytes([
            decoded[index],
            decoded[index + 1],
            decoded[index + 2],
            decoded[index + 3],
        ]) as usize;
        index += 4;
        let d = BigUint::from_bytes_be(&decoded[index..index + d_len]);

        // We're ignoring iqmp, p, and q for simplicity, but a full implementation should use these

        RsaPrivateKey::from_components(n, e, d, vec![])
            .map_err(|e| ApplicationError::from(EncryptionError::RsaError(e)))
    }

    pub fn encrypt_string(
        &self,
        data: &str,
    ) -> Result<(String, String), ApplicationError> {
        let sym_key = self.generate_symmetric_key();
        let encrypted_sym_key = self
            .encrypt_symmetric_key(&sym_key)
            .map_err(EncryptionError::from)?;
        let encrypted_data = self
            .encrypt_data(data.as_bytes(), &sym_key)
            .map_err(EncryptionError::from)?;
        Ok((
            general_purpose::STANDARD.encode(encrypted_data),
            general_purpose::STANDARD.encode(encrypted_sym_key),
        ))
    }

    pub fn decrypt_string(
        &self,
        encrypted_data: &str,
        encrypted_key: &str,
    ) -> Result<String, ApplicationError> {
        let encrypted_data = general_purpose::STANDARD
            .decode(encrypted_data)
            .map_err(EncryptionError::from)?;
        let encrypted_sym_key = general_purpose::STANDARD
            .decode(encrypted_key)
            .map_err(EncryptionError::from)?;
        let sym_key = self
            .decrypt_symmetric_key(&encrypted_sym_key)
            .map_err(EncryptionError::from)?;
        let decrypted_data = self
            .decrypt_data(&encrypted_data, &sym_key)
            .map_err(EncryptionError::from)?;
        String::from_utf8(decrypted_data)
            .map_err(EncryptionError::from)
            .map_err(ApplicationError::from)
    }

    fn generate_symmetric_key(&self) -> [u8; 32] {
        let rng = SystemRandom::new();
        let mut key = [0u8; 32];
        rng.fill(&mut key).expect("Failed to generate random key");
        key
    }

    fn encrypt_symmetric_key(
        &self,
        sym_key: &[u8],
    ) -> Result<Vec<u8>, EncryptionError> {
        let padding = Pkcs1v15Encrypt;
        let mut rng = rsa::rand_core::OsRng;
        self.public_key
            .encrypt(&mut rng, padding, sym_key)
            .map_err(EncryptionError::from)
    }

    fn decrypt_symmetric_key(
        &self,
        enc_sym_key: &[u8],
    ) -> Result<Vec<u8>, EncryptionError> {
        let padding = Pkcs1v15Encrypt;
        self.private_key
            .decrypt(padding, enc_sym_key)
            .map_err(EncryptionError::from)
    }

    fn encrypt_data(
        &self,
        data: &[u8],
        key: &[u8],
    ) -> Result<Vec<u8>, EncryptionError> {
        let key = aead::UnboundKey::new(&aead::AES_256_GCM, key)?;
        let mut sealing_key = aead::LessSafeKey::new(key);

        let nonce = aead::Nonce::assume_unique_for_key([0u8; 12]);
        let mut in_out = data.to_vec();
        sealing_key.seal_in_place_append_tag(
            nonce,
            aead::Aad::empty(),
            &mut in_out,
        )?;

        Ok(in_out)
    }

    fn decrypt_data(
        &self,
        encrypted_data: &[u8],
        key: &[u8],
    ) -> Result<Vec<u8>, EncryptionError> {
        let key = aead::UnboundKey::new(&aead::AES_256_GCM, key)?;
        let opening_key = aead::LessSafeKey::new(key);

        let nonce = aead::Nonce::assume_unique_for_key([0u8; 12]);
        let mut in_out = encrypted_data.to_vec();
        let decrypted_data = opening_key.open_in_place(
            nonce,
            aead::Aad::empty(),
            &mut in_out,
        )?;

        Ok(decrypted_data.to_vec())
    }

    pub fn sha256_hash(s: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(s.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn get_private_key_hash(
        private_key_path: &PathBuf,
    ) -> Result<String, ApplicationError> {
        let handler = EncryptionHandler::new_from_path(private_key_path)?
            .ok_or_else(|| {
                ApplicationError::EncryptionError(EncryptionError::InvalidKey(
                    "Encryption handler required to get private key hash"
                        .to_string(),
                ))
            })?;

        let key_data = handler.get_private_key_pem().map_err(|e| {
            ApplicationError::EncryptionError(EncryptionError::InvalidKey(
                e.to_string(),
            ))
        })?;
        Ok(Self::sha256_hash(&key_data))
    }
}

impl EncryptionHandler {
    pub fn load_or_generate_private_key(
        key_dir: &PathBuf,
        bits: usize,
        key_name: &str,
        password: Option<&str>,
    ) -> Result<Self, ApplicationError> {
        let key_path = key_dir.join(format!("{}.pem", key_name));

        // Try to load the existing key
        if key_path.exists() {
            let private_key_pem = fs::read_to_string(&key_path)
                .map_err(|e| ApplicationError::IOError(e))?;

            let private_key = match password {
                Some(pass) => RsaPrivateKey::from_pkcs8_encrypted_pem(
                    &private_key_pem,
                    pass,
                )
                .map_err(|e| {
                    ApplicationError::EncryptionError(
                        EncryptionError::RsaError(rsa::Error::Pkcs8(e)),
                    )
                })?,
                None => RsaPrivateKey::from_pkcs8_pem(&private_key_pem)
                    .map_err(|e| {
                        ApplicationError::EncryptionError(
                            EncryptionError::RsaError(rsa::Error::Pkcs8(e)),
                        )
                    })?,
            };

            let public_key = RsaPublicKey::from(&private_key);
            let sha256_hash = Self::sha256_hash(&private_key_pem);

            log::info!(
                "Existing RSA private key loaded successfully from {:?}",
                key_path
            );

            return Ok(Self {
                public_key,
                private_key,
                key_path,
                sha256_hash,
            });
        }

        // If loading fails, generate a new key
        Self::generate_private_key(key_dir, bits, Some(key_name), password)
    }

    pub fn generate_private_key(
        key_dir: &PathBuf,
        bits: usize,
        key_name: Option<&str>,
        password: Option<&str>,
    ) -> Result<Self, ApplicationError> {
        // Generate a new RSA private key
        let private_key = RsaPrivateKey::new(&mut rsa::rand_core::OsRng, bits)
            .map_err(|e| {
                ApplicationError::EncryptionError(EncryptionError::RsaError(e))
            })?;

        // Derive the public key
        let public_key = RsaPublicKey::from(&private_key);

        // Convert private key to PEM format, with optional encryption
        let private_key_pem = match password {
            Some(pass) => private_key
                .to_pkcs8_encrypted_pem(
                    &mut rsa::rand_core::OsRng,
                    pass.as_bytes(),
                    LineEnding::LF,
                )
                .map_err(|e| {
                    ApplicationError::EncryptionError(
                        EncryptionError::Pkcs8Error(e.into()),
                    )
                })?,
            None => private_key.to_pkcs8_pem(LineEnding::LF).map_err(|e| {
                ApplicationError::EncryptionError(EncryptionError::Pkcs8Error(
                    e.into(),
                ))
            })?,
        };

        // Ensure the directory exists
        fs::create_dir_all(key_dir)
            .map_err(|e| ApplicationError::IOError(e))?;

        let sha256_hash = Self::sha256_hash(&private_key_pem);
        let key_path = if let Some(name) = key_name {
            key_dir.join(format!("{}.pem", name))
        } else {
            key_dir.join(format!(
                "{}.pem",
                sha256_hash.chars().take(8).collect::<String>()
            ))
        };

        // Save private key to file
        fs::write(&key_path, private_key_pem.as_bytes())
            .map_err(|e| ApplicationError::IOError(e))?;

        if password.is_some() {
            log::info!(
                "RSA private key (password-protected) generated and saved \
                 successfully at {:?}",
                key_path
            );
        } else {
            log::info!(
                "RSA private key generated and saved successfully at {:?}",
                key_path
            );
        }

        // Create and return a new EncryptionHandler instance
        Ok(Self {
            public_key,
            private_key,
            key_path: key_path.clone(),
            sha256_hash,
        })
    }
}
