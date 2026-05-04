mod password_store;
mod args;

use std::process::ExitCode;
use args::Args;


fn main() -> ExitCode {
	let args = Args::gather(&[
		('h', Some("--help"), false),
	]);
	ExitCode::SUCCESS
}

fn print_help(){
	println!("usage: passwdmgr <command> [options]");
	println!("commands:");
	println!("	open [path] : Opens the password store at the path provided, or the default one");
	println!("	new [path] : Creates a password store at the path provided, or at the default location");
	println!("		-f,--force : Create a new one even if one already exists");
}
