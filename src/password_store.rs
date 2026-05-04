use crypto::Crypto;
use std::io;
use std::path::Path;

pub struct PasswordStore {
	passwords: Vec<PasswordStoreEntry>,
	crypto_algorithm: Crypto,
}

pub struct PasswordStoreEntry {
}

impl PasswordStore {
	pub fn open<T: AsRef<Path>>(path: T) -> io::Result<Self> {
	}
	pub fn new<T: AsRef<Path>>(path: T, force: bool) -> io::Result<Self> {
	}
}
