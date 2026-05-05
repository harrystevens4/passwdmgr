use std::default::Default;

pub enum Crypto {
	AESPassword(String),
}
impl Default for Crypto {
	fn default() -> Crypto {
		Crypto::AESPassword(String::new())
	}
}
impl Crypto {
	pub fn from_number(val: u16, password: &str) -> Option<Crypto> {
		match val {
			0 => Some(Crypto::AESPassword(password.to_string())),
			_ => None,
		}
	}
}
