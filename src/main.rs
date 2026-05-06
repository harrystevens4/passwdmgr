#![feature(c_size_t)]

mod password_store;
mod args;
mod crypto;

use std::process::ExitCode;
use args::Args;
use std::env;
use std::path::PathBuf;
use std::io;
use std::fs;
use std::ffi::*;
use std::os::fd::AsRawFd;

use password_store::PasswordStore;

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
	ExitCode::SUCCESS
}

fn new_subcommand(global_config: &GlobalConfig, args: &[String]) -> ExitCode {
	//====== process arguments ======
	let (args,_) = Args::gather(args,&[
		('m',Some("mkdir"),false),
		('f',Some("force"),false),
	],false);
	//====== check if a file already exists ======
	if (&global_config.password_store_path).exists() {
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
		if let Err(e) = fs::create_dir_all(&global_config.password_store_path) {
			eprintln!("Error creating parent directories for password store: {e}");
			return ExitCode::FAILURE;
		}
	}
	//====== save empty password store ======
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

fn print_help(){
	println!("usage: passwdmgr [global options] <command> [command options]");
	println!("commands:");
	println!("	open : Opens the password store at the path provided, or the default one");
	println!("		-t,--test : Open then immediately close without doing anything.");
	println!("	new : Creates a password store at the path provided, or at the default location");
	println!("		-f,--force : Create a new one even if one already exists");
	println!("		-m,--mkdir : Create parent directories if they dont exist");
	println!("global options:");
	println!("	-p,--password : Provide password for encryption/decryption");
	println!("	-f,--password-store : Provide path of password store file");
}
