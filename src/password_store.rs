use crate::crypto::*;
use std::io;
use std::path::{Path,PathBuf};
use std::fs;
use std::error::Error;
use std::convert::TryInto;
use super::VERSION;

pub struct PasswordStore {
	passwords: Vec<PasswordStoreEntry>,
	path: PathBuf,
}

pub struct PasswordStoreEntry {
}

impl PasswordStore {
	pub fn open<T: AsRef<Path>>(path: T, password: &str) -> Result<Self,Box<dyn Error>> {
		//just read the whole file
		let file = fs::read(&path)?;
		//====== read header ======
		if file.len() < 64 {
			return Err(io::Error::other("password store header malformed"))?;
		}
		//struct header {
		//	uint32_t magic_number;
		//	uint64_t password_entry_count;
		//	char iv[16];
		//	char salt[16];
		//	char reserved[20];
		//}
		let magic_number = u32::from_be_bytes(file[..4].try_into()?);
		let password_entry_count = u64::from_be_bytes(file[4..12].try_into()?);
		let iv = &file[12..28];
		let salt = &file[28..44];
		//validate magic number
		let target_magic_number_bytes = [VERSION,b'p',b'w',b's'];
		let target_magic_number = u32::from_be_bytes(target_magic_number_bytes);
		if target_magic_number != magic_number {
			return Err(io::Error::other("bad magic number"))?
		}
		//====== decrypt the passwords ======
		//generate the key
		let mut salted_password = vec![];
		salted_password.extend(salt);
		salted_password.extend(password.bytes());
		let aes_key = sha256_digest(&salted_password)?;
		//pad data to aes block size
		let mut encrypted_data = Vec::from(&file[64..]);
		let required_padding = 
			if encrypted_data.len() % 16 == 0 {0}
			else {16 - (encrypted_data.len() % 16)};
		encrypted_data.extend(vec![0].repeat(required_padding));
		//decrypt
		let decrypted_data = aes_cbc(&encrypted_data,&aes_key,&iv,EncryptionMode::Decrypt)?;
		//====== construct password store struct ======
		let password_store = PasswordStore {
			passwords: vec![],
			path: path.as_ref().to_path_buf(),
		};
		Ok(password_store)
	}
	pub fn new<T: AsRef<Path>>(path: T, password: &str) -> Self {
		PasswordStore {
			passwords: vec![],
			path: path.as_ref().to_path_buf(),
		}
	}
	pub fn save(&self) -> io::Result<()> {
		let mut output_file: Vec<u8> = vec![];
		//generate iv and salt
		let iv = get_random_bytes(16)?;
		let salt = get_random_bytes(16)?;
		//header
		let target_magic_number_bytes = [VERSION,b'p',b'w',b's'];
		output_file.extend(&target_magic_number_bytes);
		let password_entry_count = u64::to_be_bytes(self.passwords.len() as u64);
		output_file.extend(&password_entry_count);
		output_file.extend(&iv);
		output_file.extend(&salt);
		output_file.extend(vec![0; 20]);
		fs::write(&self.path,&output_file)?;
		Ok(())
	}
}
