use crate::crypto::*;
use std::io;
use std::path::{Path,PathBuf};
use std::fs;
use std::error::Error;
use std::convert::TryInto;
use super::VERSION;
use std::time::{SystemTime,UNIX_EPOCH,Duration};
use std::default::Default;

#[derive(Clone)]
pub struct PasswordStore {
	passwords: Vec<PasswordStoreEntry>,
	encryption_password: String,
	path: PathBuf,
}

#[derive(Clone)]
pub struct PasswordStoreEntry {
	time_added: SystemTime,
	identifier: String,
	username: String,
	password: String,
	notes: String,
}

impl Default for PasswordStoreEntry {
	fn default() -> Self {
		PasswordStoreEntry {
			time_added: SystemTime::now(),
			identifier: String::new(),
			username: String::new(),
			password: String::new(),
			notes: String::new(),
		}
	}
}

impl IntoIterator for PasswordStore {
	type Item = PasswordStoreEntry;
	type IntoIter = std::vec::IntoIter<PasswordStoreEntry>;

	fn into_iter(self) -> Self::IntoIter {
		self.passwords.into_iter()
	}
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
		//====== verify decryption ======
		let mut iv_password = iv.to_vec();
		iv_password.extend(password.bytes());
		let iv_password_hash = sha256_digest(&iv_password)?;
		iv_password_hash.len();
		if decrypted_data.len() < 32 {
			Err(io::Error::other("verification hash too small"))?
		}
		if iv_password_hash.as_slice() != &decrypted_data[..32] {
			Err(io::Error::other("decryption failed"))?
		}
		//====== read password entries ======
		let mut password_entries = vec![];
		let password_entry_data = &decrypted_data[32..];
		let mut i = 0;
		loop {
			if password_entries.len() == password_entry_count as usize {break}
			//====== read header ======
			if password_entry_data.len()-i < 16 {break}
			let identifier_len = u16::from_be_bytes(password_entry_data[i..(i+2)].try_into()?) as usize;
			let username_len = u16::from_be_bytes(password_entry_data[(i+2)..(i+4)].try_into()?) as usize;
			let password_len = u16::from_be_bytes(password_entry_data[(i+4)..(i+6)].try_into()?) as usize;
			let notes_len = u16::from_be_bytes(password_entry_data[(i+6)..(i+8)].try_into()?) as usize;
			let timestamp = u64::from_be_bytes(password_entry_data[(i+8)..(i+16)].try_into()?);
			i += 16;
			//====== read the data ======
			let data_len = identifier_len + username_len + notes_len;
			if password_entry_data.len()-i < data_len {break}
			//identifier
			let identifier = String::from_utf8_lossy(&password_entry_data[(i)..(i+identifier_len)]).into_owned();
			i += identifier_len;
			//username
			let username = String::from_utf8_lossy(&password_entry_data[(i)..(i+username_len)]).into_owned();
			i += username_len;
			//password
			let password = String::from_utf8_lossy(&password_entry_data[(i)..(i+password_len)]).into_owned();
			i += password_len;
			//notes
			let notes = String::from_utf8_lossy(&password_entry_data[(i)..(i+notes_len)]).into_owned();
			i += notes_len;
			//====== construct passowrd entry struct ======
			password_entries.push(PasswordStoreEntry {
				identifier,
				username,
				password,
				notes,
				time_added: UNIX_EPOCH + Duration::from_secs(timestamp),
			});
		}
		//====== construct password store struct ======
		let password_store = PasswordStore {
			passwords: password_entries,
			encryption_password: password.to_string(),
			path: path.as_ref().to_path_buf(),
		};
		Ok(password_store)
	}
	pub fn new<T: AsRef<Path>>(path: T, password: &str) -> Self {
		PasswordStore {
			passwords: vec![],
			encryption_password: password.to_string(),
			path: path.as_ref().to_path_buf(),
		}
	}
	pub fn save(&self) -> io::Result<()> {
		let mut output_file: Vec<u8> = vec![];
		let mut data_to_encrypt = vec![];
		//generate iv and salt
		let iv = get_random_bytes(16)?;
		let salt = get_random_bytes(16)?;
		//====== header ======
		let target_magic_number_bytes = [VERSION,b'p',b'w',b's'];
		output_file.extend(&target_magic_number_bytes);
		let password_entry_count = u64::to_be_bytes(self.passwords.len() as u64);
		output_file.extend(&password_entry_count);
		output_file.extend(&iv);
		output_file.extend(&salt);
		output_file.extend(vec![0; 20]);
		//====== verification hash ======
		let mut iv_password = iv.to_vec();
		iv_password.extend(self.encryption_password.bytes());
		let iv_password_hash = sha256_digest(&iv_password)?;
		data_to_encrypt.extend(&iv_password_hash);
		//====== password entries ======
		//struct password_entry {
		//	uint16_t identifier_len;
		//	uint16_t username_len;
		//	uint16_t password_len;
		//	uint16_t notes_len;
		//	uint64_t time_added;
		//	char identifier[];
		//	char username[];
		//	char password[];
		//	char notes[];
		//}
		for password_entry in &self.passwords {
			//====== prepare all the struct fields ======
			let mut password_entry_buffer = vec![];
			let identifier_len: u16 = password_entry.identifier
				.len()
				.try_into()
				.unwrap_or(u16::MAX);
			let username_len: u16 = password_entry.username
				.len()
				.try_into()
				.unwrap_or(u16::MAX);
			let password_len: u16 = password_entry.password
				.len()
				.try_into()
				.unwrap_or(u16::MAX);
			let notes_len: u16 = password_entry.notes
				.len()
				.try_into()
				.unwrap_or(u16::MAX);
			let identifier = slice_take(password_entry.identifier.as_bytes(),identifier_len);
			let username = slice_take(password_entry.username.as_bytes(),username_len);
			let password = slice_take(password_entry.password.as_bytes(),password_len);
			let notes = slice_take(password_entry.notes.as_bytes(),notes_len);
			let timestamp: u64 = password_entry.time_added
				.duration_since(UNIX_EPOCH)
				.unwrap_or(Duration::new(0,0))
				.as_secs();
			//====== append all the data ======
			password_entry_buffer.extend(identifier_len.to_be_bytes());
			password_entry_buffer.extend(username_len.to_be_bytes());
			password_entry_buffer.extend(password_len.to_be_bytes());
			password_entry_buffer.extend(notes_len.to_be_bytes());
			password_entry_buffer.extend(timestamp.to_be_bytes());
			password_entry_buffer.extend(identifier);
			password_entry_buffer.extend(username);
			password_entry_buffer.extend(password);
			password_entry_buffer.extend(notes);
			data_to_encrypt.extend(&password_entry_buffer);
		}
		//====== encrypt data ======
		//generate the key
		let mut salted_password = vec![];
		salted_password.extend(salt);
		salted_password.extend(self.encryption_password.bytes());
		let aes_key = sha256_digest(&salted_password)?;
		//pad data to aes block size
		let required_padding = 
			if data_to_encrypt.len() % 16 == 0 {0}
			else {16 - (data_to_encrypt.len() % 16)};
		data_to_encrypt.extend(vec![0].repeat(required_padding));
		//encrypt
		let encrypted_data = aes_cbc(&data_to_encrypt,&aes_key,&iv,EncryptionMode::Encrypt)?;
		output_file.extend(&encrypted_data);
		//====== write to file ======
		fs::write(&self.path,&output_file)?;
		Ok(())
	}
	pub fn entries(&self) -> Vec<PasswordStoreEntry> {
		self.passwords.clone()
	}
	pub fn add_entry(&mut self, entry: PasswordStoreEntry){
		self.passwords.push(entry);
	}
}

impl PasswordStoreEntry {
	pub fn time_added(&self) -> SystemTime {self.time_added.clone()}
	pub fn identifier(&self) -> String {self.identifier.clone()}
	pub fn username(&self) -> String {self.username.clone()}
	pub fn password(&self) -> String {self.password.clone()}
	pub fn notes(&self) -> String {self.notes.clone()}

	pub fn new(identifier: &str, username: &str, password: &str, notes: &str) -> Self {
		let mut entry = PasswordStoreEntry::default();
		entry.identifier = identifier.to_string();
		entry.username = username.to_string();
		entry.password = password.to_string();
		entry.notes = notes.to_string();
		entry
	}
}

fn slice_take<T: Into<usize> + Copy,U>(slice: &[U], n: T) -> &[U] {
	if slice.len() > n.into() {
		&slice[..(n.into())]
	}else {
		slice
	}
}
