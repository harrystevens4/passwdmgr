#![feature(c_size_t)]

mod password_store;
mod args;
mod crypto;
mod ncurses;
mod constants;
mod tui;

use std::process::ExitCode;
use args::Args;
use std::env;
use std::path::PathBuf;
use std::io;
use std::fs;
use std::ffi::*;
use std::os::fd::AsRawFd;
use std::time::Duration;

use password_store::{PasswordStore,PasswordStoreEntry};
use crate::tui::run_tui;

struct GlobalConfig {
	password: Option<String>,
	password_store_path: PathBuf,
}

const VERSION: u8 = 1;

unsafe extern "C" {
	fn tty_set_echo(fd: c_int, state: c_int) -> c_int;
}

fn main() -> ExitCode {
	//====== get arguments and split into global and subcommand ======
	let global_arginfo = [
		('h', Some("help"), false),
		('p', Some("password"), true),
		('f', Some("password-store"), true),
	];
	let mut args: Vec<String> = env::args().collect();
	let _ = args.remove(0); //remove argv[0]
	//find first subcommand
	let (global_args,subcommand_args) = Args::gather(&args,&global_arginfo,true);
	//====== process global arguments and commands ======
	if global_args.has('h') {
		print_help();
		return ExitCode::SUCCESS;
	}
	if global_args.others().len() < 1 {
		eprintln!("Please provide a command. See --help for help");
		return ExitCode::FAILURE;
	}
	//if user path not provided default to ~/.local/share/passwdmgr/passwords
	//then default to /usr/share/passwdmgr/passwords if no homedir found
	let password_store_path = global_args
		.get_value('f')
		.map(PathBuf::from)
		.unwrap_or(env::home_dir()
			.map(|home| home.join(".local/share/passwdmgr/passwords"))
			.unwrap_or(PathBuf::from("/usr/share/passwdmgr/passwords"))
		);
	//global configuration options
	let global_config = GlobalConfig {
		password: global_args.get_value('p'),
		password_store_path,
	};
	//====== breakout into seperate functions for each subcommand ======
	match global_args.others()[0].as_str() {
		"open" => open_subcommand(&global_config,&subcommand_args),
		"new" => new_subcommand(&global_config,&subcommand_args),
		"test" => test_subcommand(),
		"print" => print_subcommand(&global_config,&subcommand_args),
		"add" => add_subcommand(&global_config,&subcommand_args),
		subcommand => {
			eprintln!("Bad subcommand \"{}\", use --help for help",subcommand);
			ExitCode::FAILURE
		}
	}
}

fn open_subcommand(global_config: &GlobalConfig, args: &[String]) -> ExitCode {
	//====== collect arguments ======
	let (args,_) = Args::gather(args,&[
		('t',Some("test"),false),
	],false);
	//====== open password store ======
	let password_prompt = format!("Enter password for {}:",global_config.password_store_path.to_string_lossy());
	let password = global_config.password
		.clone()
		.unwrap_or_else(|| prompt_for_password(&password_prompt));
	println!("opening password store...");
	let password_store = match PasswordStore::open(&global_config.password_store_path,&password) {
		Ok(p) => p, Err(e) => {
			eprintln!("Error opening password store at \"{}\": {}",global_config.password_store_path.to_string_lossy(),e);
			return ExitCode::FAILURE;
		}
	};
	//test mode simply opens then closes it
	if args.has('t') {return ExitCode::SUCCESS}
	run_tui(&password_store);
	ExitCode::SUCCESS
}

fn new_subcommand(global_config: &GlobalConfig, args: &[String]) -> ExitCode {
	//====== process arguments ======
	let (args,_) = Args::gather(args,&[
		('m',Some("mkdir"),false),
		('f',Some("force"),false),
	],false);
	//====== check if a file already exists ======
	if (&global_config.password_store_path).exists() && !args.has('f') {
		eprintln!("password store already exists, use --force to ignore");
		return ExitCode::FAILURE;
	}
	//====== create password store object ======
	println!("creating new password store...");
	let password = global_config.password
		.clone()
		.unwrap_or_else(|| prompt_for_password("Enter new password:"));
	let password_store = PasswordStore::new(&global_config.password_store_path,&password);
	//====== create directories if requested ======
	if args.has('m') {
		if let Some(parent_dirs) = global_config.password_store_path.parent() {
			if let Err(e) = fs::create_dir_all(parent_dirs) {
				eprintln!("Error creating parent directories for password store: {e}");
				return ExitCode::FAILURE;
			}
		}
	}
	//====== save empty password store ======
	if let Err(e) = password_store.save() {
		eprintln!("Error saving password store: {e}");
		return ExitCode::FAILURE;
	}
	ExitCode::SUCCESS
}

fn print_subcommand(global_config: &GlobalConfig, args: &[String]) -> ExitCode {
	//====== process arguments ======
	let (args,_) = Args::gather(args,&[
		('t',Some("sort-by-time"),false),
		('i',Some("sort-by-identifier"),false),
		('n',Some("sort-by-username"),false),
		('r',Some("reverse"),false),
	],false);
	//====== open store ======
	let password_prompt = format!("Enter password for {}:",global_config.password_store_path.to_string_lossy());
	let password = global_config.password
		.clone()
		.unwrap_or_else(|| prompt_for_password(&password_prompt));
	println!("opening password store...");
	let password_store = match PasswordStore::open(&global_config.password_store_path,&password) {
		Ok(p) => p, Err(e) => {
			eprintln!("Error opening password store at \"{}\": {}",global_config.password_store_path.to_string_lossy(),e);
			return ExitCode::FAILURE;
		}
	};
	//====== sort ======
	let mut stored_passwords = password_store.entries();
	if args.has('n') {stored_passwords.sort_by_key(|p: &PasswordStoreEntry| p.username())}
	else if args.has('i') {stored_passwords.sort_by_key(|p: &PasswordStoreEntry| p.identifier())}
	else {stored_passwords.sort_by_key(|p: &PasswordStoreEntry| p.time_added())}
	//reverse if descending required
	if args.has('r') {stored_passwords.reverse()}
	//====== print ======
	for entry in &stored_passwords {
		println!("[{}]",entry.identifier());
		println!("username: \"{}\"",entry.username());
		println!("password: \"{}\"",entry.password());
		println!("notes: \"{}\"",entry.notes());
		println!("");
	}
	if stored_passwords.len() == 0 {println!("No passwords stored.")}
	ExitCode::SUCCESS
}

fn add_subcommand(global_config: &GlobalConfig, args: &[String]) -> ExitCode {
	//====== process arguments ======
	let (args,_) = Args::gather(args,&[
		('i',Some("identifier"),true),
		('u',Some("username"),true),
		('p',Some("password"),true),
		('n',Some("notes"),true),
	],false);
	//====== open store ======
	let password_prompt = format!("Enter password for {}:",global_config.password_store_path.to_string_lossy());
	let password = global_config.password
		.clone()
		.unwrap_or_else(|| prompt_for_password(&password_prompt));
	println!("opening password store...");
	let mut password_store = match PasswordStore::open(&global_config.password_store_path,&password) {
		Ok(p) => p, Err(e) => {
			eprintln!("Error opening password store at \"{}\": {}",global_config.password_store_path.to_string_lossy(),e);
			return ExitCode::FAILURE;
		}
	};
	//====== prompt for info ======
	let get_field = |field: &str|{
		println!("enter {}:",field);
		let stdin = io::stdin();
		let mut buffer = String::new();
		let _ = stdin.read_line(&mut buffer);
		buffer
			.trim_end_matches('\n')
			.to_string()
	};
	let identifier = args.get_value('i').unwrap_or_else(|| get_field("identifier"));
	let username = args.get_value('u').unwrap_or_else(|| get_field("username"));
	let password = args.get_value('p').unwrap_or_else(|| get_field("password"));
	let notes = args.get_value('p').unwrap_or_else(|| get_field("notes"));
	//====== add to password store ======
	let entry = PasswordStoreEntry::new(&identifier,&username,&password,&notes);
	password_store.add_entry(entry);
	//====== save ======
	if let Err(e) = password_store.save() {
		eprintln!("Error saving password store: {e}");
		return ExitCode::FAILURE;
	}
	ExitCode::SUCCESS
}

fn prompt_for_password(prompt: &str) -> String {
	let stdin = io::stdin();
	//echo off
	unsafe {tty_set_echo(stdin.as_raw_fd(),0)};
	//show prompt
	println!("{prompt}");
	let mut line_buffer = String::new();
	let _ = stdin.read_line(&mut line_buffer);
	//echo on
	unsafe {tty_set_echo(stdin.as_raw_fd(),1)};
	//return the password
	line_buffer.trim_end_matches('\n').to_string()
}

fn test_subcommand() -> ExitCode {
	use crate::crypto::*;
	println!("sha256 test:");
	assert_eq!(
		vec![0x81_u8,0x71,0xea,0x07,0x8b,0x3f,0xd0,0x25,0x56,0x9b,0x77,0xc2,0x77,0x72,0x8c,0xfb,0x4d,0xe8,0x7b,0xd4,0x40,0x0b,0xe8,0x43,0x62,0xab,0x65,0x8c,0x0e,0x1e,0x67,0xac,],
		sha256_digest(&"kjhfads34987bjn234".bytes().collect::<Vec<u8>>()).unwrap()
	);
	println!("passed");
	println!("aes test:");
	let iv = get_random_bytes(16).unwrap();
	let key = get_random_bytes(32).unwrap();
	let data = get_random_bytes(32).unwrap().repeat(100);
	let encrypted_data = aes_cbc(&data,&key,&iv,EncryptionMode::Encrypt).unwrap();
	let decrypted_data = aes_cbc(&encrypted_data,&key,&iv,EncryptionMode::Decrypt).unwrap();
	assert_eq!(data,decrypted_data);
	println!("passed");
	ExitCode::SUCCESS
}

fn print_help(){
	println!("usage: passwdmgr [global options] <command> [command options]");
	println!("commands:");
	println!("    open : Opens the password store at the path provided, or the default one");
	println!("        -t,--test : Open then immediately close without doing anything.");
	println!("");
	println!("    new : Creates a password store at the path provided, or at the default location");
	println!("        -f,--force : Create a new one even if one already exists");
	println!("        -m,--mkdir : Create parent directories if they dont exist");
	println!("");
	println!("    print : Print all passwords in a store");
	println!("        -t,--sort-by-time       : Sort by time added (ascending)");
	println!("        -i,--sort-by-identifier : Sort by identifier (ascending)");
	println!("        -n,--sort-by-username   : Sort by username (ascending)");
	println!("        -r,--reverse            : Reverse order (sort by descending)");
	println!("");
	println!("global options:");
	println!("    -p,--password : Provide password for encryption/decryption");
	println!("    -f,--password-store : Provide path of password store file");
}
