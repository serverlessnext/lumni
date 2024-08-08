use base64::engine::general_purpose;
use base64::Engine as _;
use lumni::api::error::{ApplicationError, EncryptionError};
use ring::aead;
use ring::rand::{SecureRandom, SystemRandom};
use rsa::pkcs8::{
    DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey,
    LineEnding,
};
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};

use crate::external as lumni;

pub struct EncryptionHandler {
    public_key: RsaPublicKey,
    private_key: RsaPrivateKey,
}

impl EncryptionHandler {
    pub fn new(
        public_key_pem: &str,
        private_key_pem: &str,
    ) -> Result<Self, ApplicationError> {
        let public_key = RsaPublicKey::from_public_key_pem(public_key_pem)
            .map_err(EncryptionError::from)?;
        let private_key = RsaPrivateKey::from_pkcs8_pem(private_key_pem)
            .map_err(EncryptionError::from)?;
        Ok(Self {
            public_key,
            private_key,
        })
    }

    pub fn get_ssh_private_key(&self) -> Result<Vec<u8>, EncryptionError> {
        // Convert the RSA private key to PKCS#8 PEM format with LF line endings
        let pem = self
            .private_key
            .to_pkcs8_pem(LineEnding::LF)
            .map_err(EncryptionError::from)?;

        // Convert the PEM string to bytes
        Ok(pem.as_bytes().to_vec())
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
}
