//! Encryption of notes, the session and backups.
//!
//! - Passwords become keys with Argon2id (memory-hard, so guessing is slow);
//!   salt and cost parameters are stored next to the data ([`Kdf`]).
//! - Data is sealed with XChaCha20-Poly1305: authenticated encryption, so a
//!   wrong key or tampered data is detected instead of producing garbage. A
//!   random 192-bit nonce per seal; "associated data" binds a ciphertext to
//!   its purpose (e.g. the note id), so files cannot be swapped unnoticed.
//! - Passwords are never stored. Keys are wiped from memory when dropped.
//!
//! Sealed data: `QNE1` | 24-byte nonce | ciphertext with 16-byte tag.


use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::Zeroizing;

const MAGIC: &[u8; 4] = b"QNE1";
const NONCE_LEN: usize = 24;
const TAG_LEN: usize = 16;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// Wrong password/key, or the data was changed.
    Decrypt,
    /// Not encrypted data, or unusable parameters.
    Format,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Decrypt => write!(f, "{}", t!("Wrong password, or the data is damaged")),
            Error::Format => write!(f, "{}", t!("The encrypted data is damaged")),
        }
    }
}

pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut buf = [0u8; N];
    getrandom::getrandom(&mut buf).expect("system random number generator");
    buf
}

fn b64() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

/// Argon2id settings for deriving a key from a password.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Kdf {
    /// Base64, 16 random bytes.
    pub salt: String,
    /// Memory in KiB.
    pub memory: u32,
    pub iterations: u32,
    pub lanes: u32,
}

impl Kdf {
    /// Fresh salt with the default cost: 64 MiB, 3 passes (about half a second).
    pub fn new() -> Self {
        Self::with_cost(64 * 1024, 3)
    }

    pub fn with_cost(memory: u32, iterations: u32) -> Self {
        Kdf { salt: b64().encode(random_bytes::<16>()), memory, iterations, lanes: 1 }
    }

    /// Derives the key (slow on purpose; run it off the main thread).
    pub fn derive(&self, password: &str) -> Result<Key, Error> {
        // Bounds keep crafted files from requesting absurd resources.
        if !(8..=1024 * 1024).contains(&self.memory) || !(1..=16).contains(&self.iterations) || !(1..=8).contains(&self.lanes)
        {
            return Err(Error::Format);
        }
        let salt = b64().decode(&self.salt).map_err(|_| Error::Format)?;
        let params = Params::new(self.memory, self.iterations, self.lanes, Some(32)).map_err(|_| Error::Format)?;
        let mut key = Zeroizing::new([0u8; 32]);
        Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
            .hash_password_into(password.as_bytes(), &salt, key.as_mut())
            .map_err(|_| Error::Format)?;
        Ok(Key(key))
    }
}

impl Default for Kdf {
    fn default() -> Self {
        Self::new()
    }
}

/// A 256-bit key, wiped from memory when dropped.
#[derive(Clone)]
pub struct Key(Zeroizing<[u8; 32]>);

impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Key(…)")
    }
}

impl Key {
    pub fn random() -> Self {
        Key(Zeroizing::new(random_bytes::<32>()))
    }

    fn cipher(&self) -> XChaCha20Poly1305 {
        XChaCha20Poly1305::new(self.0.as_ref().into())
    }

    /// Encrypts `plain`; `aad` must be given again to decrypt.
    pub fn seal(&self, plain: &[u8], aad: &[u8]) -> Vec<u8> {
        let nonce = random_bytes::<NONCE_LEN>();
        let ct = self
            .cipher()
            .encrypt(XNonce::from_slice(&nonce), Payload { msg: plain, aad })
            .expect("encryption cannot fail for in-memory data");
        let mut out = Vec::with_capacity(MAGIC.len() + NONCE_LEN + ct.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ct);
        out
    }

    pub fn open(&self, sealed: &[u8], aad: &[u8]) -> Result<Zeroizing<Vec<u8>>, Error> {
        if !is_sealed(sealed) || sealed.len() < MAGIC.len() + NONCE_LEN + TAG_LEN {
            return Err(Error::Format);
        }
        let (nonce, ct) = sealed[MAGIC.len()..].split_at(NONCE_LEN);
        self.cipher()
            .decrypt(XNonce::from_slice(nonce), Payload { msg: ct, aad })
            .map(Zeroizing::new)
            .map_err(|_| Error::Decrypt)
    }

    /// Text variant of [`Key::seal`] (Base64).
    pub fn seal_text(&self, plain: &str, aad: &[u8]) -> String {
        b64().encode(self.seal(plain.as_bytes(), aad))
    }

    pub fn open_text(&self, sealed: &str, aad: &[u8]) -> Result<Zeroizing<String>, Error> {
        let data = b64().decode(sealed.trim()).map_err(|_| Error::Format)?;
        let plain = self.open(&data, aad)?;
        String::from_utf8(plain.to_vec()).map(Zeroizing::new).map_err(|_| Error::Format)
    }

    /// The raw key (Base64), for a secret store such as the keyring.
    pub fn export(&self) -> Zeroizing<String> {
        Zeroizing::new(b64().encode(self.0.as_ref()))
    }

    pub fn import(encoded: &str) -> Result<Key, Error> {
        let bytes = Zeroizing::new(b64().decode(encoded.trim()).map_err(|_| Error::Format)?);
        let bytes: [u8; 32] = bytes.as_slice().try_into().map_err(|_| Error::Format)?;
        Ok(Key(Zeroizing::new(bytes)))
    }

    /// This key encrypted with `kek` (Base64), for storing it.
    pub fn wrap(&self, kek: &Key) -> String {
        b64().encode(kek.seal(self.0.as_ref(), b"quicknotes-key"))
    }

    pub fn unwrap(wrapped: &str, kek: &Key) -> Result<Key, Error> {
        let data = b64().decode(wrapped.trim()).map_err(|_| Error::Format)?;
        let plain = kek.open(&data, b"quicknotes-key")?;
        let bytes: [u8; 32] = plain.as_slice().try_into().map_err(|_| Error::Format)?;
        Ok(Key(Zeroizing::new(bytes)))
    }
}

pub fn is_sealed(data: &[u8]) -> bool {
    data.starts_with(MAGIC)
}

/// A password-protected key as stored on disk: the Argon2id settings and the
/// random data key wrapped with the password's key. Changing the password
/// only re-wraps the data key.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Lock {
    pub kdf: Kdf,
    /// The data key, sealed with the key derived from the password.
    pub key: String,
}

impl Lock {
    /// A new random data key protected by `password` (slow).
    pub fn create(password: &str, kdf: Kdf) -> (Lock, Key) {
        let data_key = Key::random();
        let kek = kdf.derive(password).expect("fresh parameters are valid");
        (Lock { kdf, key: data_key.wrap(&kek) }, data_key)
    }

    /// The data key, if `password` is right (slow).
    pub fn open(&self, password: &str) -> Result<Key, Error> {
        Key::unwrap(&self.key, &self.kdf.derive(password)?)
    }

    /// The same data key protected by a new password (slow).
    pub fn rewrap(data_key: &Key, new_password: &str, kdf: Kdf) -> Lock {
        let kek = kdf.derive(new_password).expect("fresh parameters are valid");
        Lock { kdf, key: data_key.wrap(&kek) }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// Cheap parameters: Argon2 with 64 MiB is slow in debug builds.
    pub fn fast_kdf() -> Kdf {
        Kdf::with_cost(8, 1)
    }

    #[test]
    fn seal_and_open() {
        let key = Key::random();
        let sealed = key.seal(b"secret note", b"id1");
        assert!(is_sealed(&sealed));
        assert!(!sealed.windows(6).any(|w| w == b"secret"));
        assert_eq!(key.open(&sealed, b"id1").unwrap().as_slice(), b"secret note");
        // Other purpose, other key, tampered data.
        assert_eq!(key.open(&sealed, b"id2"), Err(Error::Decrypt));
        assert_eq!(Key::random().open(&sealed, b"id1"), Err(Error::Decrypt));
        let mut bad = sealed.clone();
        *bad.last_mut().unwrap() ^= 1;
        assert_eq!(key.open(&bad, b"id1"), Err(Error::Decrypt));
        assert_eq!(key.open(b"plain text", b"id1"), Err(Error::Format));
        // Same input, different nonce.
        assert_ne!(key.seal(b"x", b""), key.seal(b"x", b""));
    }

    #[test]
    fn text_round_trip() {
        let key = Key::random();
        let sealed = key.seal_text("Grüße 👋", b"session");
        assert_eq!(key.open_text(&sealed, b"session").unwrap().as_str(), "Grüße 👋");
    }

    #[test]
    fn passwords() {
        let kdf = fast_kdf();
        assert_eq!(format!("{:?}", kdf.derive("pw").unwrap()), "Key(…)");
        let (lock, key) = Lock::create("correct horse", kdf);
        let sealed = key.seal(b"data", b"");
        assert_eq!(lock.open("wrong").unwrap_err(), Error::Decrypt);
        let opened = lock.open("correct horse").unwrap();
        assert_eq!(opened.open(&sealed, b"").unwrap().as_slice(), b"data");
        // A new password keeps the data key.
        let relocked = Lock::rewrap(&opened, "new password", fast_kdf());
        assert!(relocked.open("correct horse").is_err());
        assert_eq!(relocked.open("new password").unwrap().open(&sealed, b"").unwrap().as_slice(), b"data");
    }

    #[test]
    fn keys_export_and_import() {
        let key = Key::random();
        let sealed = key.seal(b"x", b"");
        let back = Key::import(&key.export()).unwrap();
        assert_eq!(back.open(&sealed, b"").unwrap().as_slice(), b"x");
        assert_eq!(Key::import("too short").unwrap_err(), Error::Format);
    }

    #[test]
    fn rejects_absurd_parameters() {
        let kdf = Kdf { memory: u32::MAX, ..fast_kdf() };
        assert_eq!(kdf.derive("x").unwrap_err(), Error::Format);
    }
}
