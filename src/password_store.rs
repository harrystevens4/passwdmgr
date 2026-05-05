use crypto::Crypto;
use std::io;
use std::path::{Path,PathBuf};
use std::fs;
use std::mem;
use std::error::Error;
use std::convert::TryInto;
use super::VERSION;

pub struct PasswordStore {
	passwords: Vec<PasswordStoreEntry>,
	crypto_algorithm: Crypto,
	path: PathBuf,
}

pub struct PasswordStoreEntry {
}

impl PasswordStore {
	pub fn open<T: AsRef<Path>>(path: T, password: &str) -> Result<Self,Box<dyn Error>> {
		//just read the whole file
		let file = fs::read(&path)?;
		//====== read header ======
		if file.len() < 30 {
			return Err(io::Error::other("password store header malformed"))?;
		}
		//struct header {
		//	uint32_t magic_number;
		//	uint16_t encryption_algorithm;
		//	uint64_t encrypted_size; //encryption block
		//	uint64_t decrypted_size; //encryption block
		//	uint64_t password_entry_count;
		//}
		let magic_number = u32::from_be_bytes(file[..4].try_into()?);
		let encryption_algorithm = u16::from_be_bytes(file[4..6].try_into()?);
		let encrypted_size = u64::from_be_bytes(file[6..14].try_into()?);
		let decrypted_size = u64::from_be_bytes(file[14..22].try_into()?);
		let password_entry_count = u64::from_be_bytes(file[22..30].try_into()?);
		//validate magic number
		let target_magic_number_bytes = [VERSION,b'p',b'w',b's'];
		let target_magic_number = u32::from_be_bytes(target_magic_number_bytes);
		if target_magic_number != magic_number {
			return Err(io::Error::other("bad magic number"))?
		}
		//====== construct password store struct ======
		let mut password_store = PasswordStore {
			passwords: vec![],
			crypto_algorithm: 
				Crypto::from_number(encryption_algorithm,&password)
				.ok_or(io::Error::other("Invalid Cryptography algorithm"))?,
			path: path.as_ref().to_path_buf(),
		};
		Ok(password_store)
	}
	pub fn new<T: AsRef<Path>>(path: T, password: &str) -> Result<Self,Box<dyn Error>> {
		Ok(PasswordStore {
			passwords: vec![],
			crypto_algorithm: Crypto::AESPassword(password.to_string()),
			path: path.as_ref().to_path_buf(),
		})
	}
}
