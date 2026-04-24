use blake3::Hasher;
use chrono::Utc;

use crate::CredentialSecret;

pub(crate) fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

pub(crate) fn credential_digest(salt: &[u8], secret: &CredentialSecret) -> Vec<u8> {
    let mut hasher = Hasher::new();
    hasher.update(salt);
    hasher.update(secret.expose_secret().as_bytes());
    hasher.finalize().as_bytes().to_vec()
}
