#![feature(c_size_t)]

mod password_store;
mod args;
mod crypto;
mod ncurses;
mod constants;

use std::process::ExitCode;
use args::Args;
use std::env;
use std::path::PathBuf;
use std::io;
use std::fs;
use std::ffi::*;
use std::os::fd::AsRawFd;
use std::time::Duration;
use std::cmp::{min,max};

use password_store::{PasswordStore,PasswordStoreEntry};
use ncurses::{Ncurses,Input,Window,VideoAttribute};

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
	//====== ncurses initialisation ======
	let ncurses = Ncurses::init();
	let Some(stdscr) = ncurses.stdscr()
	else {
		eprintln!("couldn't open stdscr");
		return ExitCode::FAILURE;
	};
	stdscr.keypad(true);
	ncurses.noecho();
	//create windows
	let selection_window = ncurses.newwin(1,1,0,0);
	let control_window = ncurses.newwin(1,1,0,0);
	let info_window = ncurses.newwin(1,1,0,0);
	//update window sizes
	redraw_windows(&stdscr,&selection_window,&control_window,&info_window);
	//====== key variables ======
	//seperate so when the user clears the filter they go back to where they were
	let mut unfiltered_selected_password = 0_isize; 
	let mut filtered_selected_password = 0_isize;
	let mut filter = String::new();
	//====== update loop ======
	loop {
		//====== render each window ======
		let selectable_passwords = vec!["amazon.co.uk","ebay.co.uk","netflix.co.uk","gmail.com","web.whatsapp.com","outlook.com","disneyplus.com","hotmail.com","realy-long-domain-name.co.uk","nhs.gov.uk","uk.webuy.com","accounts.firefox.com","cad.onshape.com","homebase.co.uk","nowtv.com","my.integralmaths.org","computer password","phone password","totp recovery codes"].into_iter().map(String::from).collect::<Vec<_>>();
		//let selectable_passwords = password_store
		//	.entries()
		//	.into_iter()
		//	.map(|p| p.identifier())
		//	.collect::<Vec<String>>();
		render_selections(
			&selection_window,
			&selectable_passwords,
			if filter.len() > 0 {filtered_selected_password} else {unfiltered_selected_password} as usize
		);
		selection_window.refresh();
		info_window.mvaddstr(1,1,format!("{unfiltered_selected_password} {filtered_selected_password}"));
		info_window.refresh();
		//====== input handling ======
		match ncurses.getch() {
			Input::Up => if filter.len() > 0 {filtered_selected_password-=1} else {unfiltered_selected_password-=1},
			Input::Down => if filter.len() > 0 {filtered_selected_password+=1} else {unfiltered_selected_password+=1},
			Input::Left => selection_window.mvaddstr(1,1,"left"),
			Input::Right => selection_window.mvaddstr(1,1,"right"),
			Input::Resize => redraw_windows(&stdscr,&selection_window,&control_window,&info_window),
			_ => ()
		}
		unfiltered_selected_password = unfiltered_selected_password.rem_euclid(selectable_passwords.len() as isize);//password_store.entries().len();
	}
	//====== cleanup ======
	control_window.delwin();
	selection_window.delwin();
	info_window.delwin();
	stdscr.delwin();
	ncurses.end();
	ExitCode::SUCCESS
}

fn render_selections(selection_window: &Window, selections: &[String], selection_index: usize){
	//====== figure out where the cursor and all the selections should be ======
	let (height,width) = selection_window.getmaxyx();
	//dont do anything
	if selections.len() == 0 || height < 2 {return}
	//I cast... 1000000 isizes!!!!!
	let cursor_index = max(
		min(selection_index,(height-2)/2) as isize,
		max((selection_index as isize) - max((selections.len() as isize),(height as isize - 2)) + (height as isize - 2),0
		)
	) as usize;
	let selection_start_index = min(
		max(0,(selection_index as isize) - ((height-2)/2) as isize),
		max((selections.len() as isize) - (height as isize - 2),0),
	) as usize;
	//====== actualy render the selections ======
	for (i,selection) in selections[selection_start_index..].iter().enumerate() {
		//dont render past the end of the window
		if i >= (height-2) {break}
		//truncate to fit
		let truncated_selection = selection
			.chars()
			.take(width-2)
			.collect::<String>();
		//add the selection
		selection_window.mvaddstr(i+1,1,format!("{:1$}",truncated_selection,width-2,)); //pad with spaces
		//remove any previous highlighting
		selection_window.mvchgat(i+1,1,(width-2) as isize,&[VideoAttribute::Normal],0);
	}
	//====== highlight the current selection ======
	selection_window.mvchgat(cursor_index+1,1,(width-2) as isize,&[VideoAttribute::Reverse],0);
}

fn redraw_windows(stdscr: &Window, selection_window: &Window, control_window: &Window, info_window: &Window){
	//====== calculate window sizes and positions ======
	let (term_height,term_width) = stdscr.getmaxyx();
	let selection_window_width = term_width/3;
	let selection_window_height = term_height;
	let selection_window_x = 0;
	let selection_window_y = 0;

	let control_window_height = term_height/3;
	let control_window_width = term_width-selection_window_width;
	let control_window_x = selection_window_width;
	let control_window_y = term_height-control_window_height;

	let info_window_width = control_window_width;
	let info_window_height = term_height-control_window_height;
	let info_window_x = selection_window_width;
	let info_window_y = 0;
	//====== erase current windows ======
	selection_window.erase();
	control_window.erase();
	info_window.erase();
	stdscr.erase();
	//====== move and resize ======
	//resize
	selection_window.resize(selection_window_height,selection_window_width);
	control_window.resize(control_window_height,control_window_width);
	info_window.resize(info_window_height,info_window_width);
	//move
	selection_window.mvwin(selection_window_y,selection_window_x);
	control_window.mvwin(control_window_y,control_window_x);
	info_window.mvwin(info_window_y,info_window_x);
	//====== redraw borders ======
	selection_window.r#box(0,0);
	control_window.r#box(0,0);
	info_window.r#box(0,0);
	//refresh
	stdscr.refresh();
	selection_window.refresh();
	control_window.refresh();
	info_window.refresh();
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
