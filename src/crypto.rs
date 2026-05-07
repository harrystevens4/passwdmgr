use std::ffi::*;
use std::io;

#[repr(C)]
struct skcipher_info {
	alg_name: *const c_char,

	message: *const c_char,
	message_len: c_size_t,

	key: *const c_char,
	key_len: c_size_t,

	iv: *const c_char,
	iv_len: c_size_t,
	
	output: *mut c_char,
	output_len: c_size_t,
}

unsafe extern "C" {
	fn skcipher(skcipher_info: *mut skcipher_info, encrypt: c_int) -> c_int;
	fn sha256(input: *const c_char, input_len: c_size_t, digest: *mut c_uchar) -> c_int;
	fn getentropy(buffer: *mut c_char, length: c_size_t) -> c_int;
}

pub enum EncryptionMode {
	Encrypt,
	Decrypt,
}

pub fn aes_cbc(input: &[u8], key: &[u8], iv: &[u8], mode: EncryptionMode) -> io::Result<Vec<u8>> {
	//====== allocate output buffer ======
	let mut output: Vec<u8> = vec![0; input.len()];
	//====== prepare cipher info ======
	let encrypt_mode: c_int = match mode {
		EncryptionMode::Encrypt => 1,
		EncryptionMode::Decrypt => 0,
	};
	let alg_name = c"cbc(aes)";
	let mut cipher_info = skcipher_info {
		alg_name: alg_name.as_ptr(),
		message: input.as_ptr() as *const i8,
		message_len: input.len(),
		key: key.as_ptr() as *const i8,
		key_len: key.len(),
		iv: iv.as_ptr() as *const i8,
		iv_len: iv.len(),
		output: output.as_mut_ptr() as *mut i8,
		output_len: output.len(),
	};
	let result = unsafe { skcipher(&mut cipher_info,encrypt_mode) };
	if result < 0 {Err(io::Error::last_os_error())?}
	Ok(output)
}

pub fn sha256_digest(input: &[u8]) -> io::Result<Vec<u8>> {
	let mut digest = vec![0; 32];
	let result = unsafe {
		sha256(
			input.as_ptr() as *const i8,
			input.len(),
			digest.as_mut_ptr()
		)
	};
	if result < 0 {Err(io::Error::last_os_error())?}
	Ok(digest)
}

pub fn get_random_bytes(count: usize) -> io::Result<Vec<u8>> {
	let mut buffer = vec![0; count];
	let result = unsafe { getentropy(buffer.as_mut_ptr(),count) };
	if result < 0 {Err(io::Error::last_os_error())?}
	Ok(buffer
		.into_iter()
		.map(|b| b as u8)
		.collect()
	)
}
