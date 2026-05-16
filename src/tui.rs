use crate::ncurses::{Ncurses,Input,Window,VideoAttribute};
use crate::password_store::{PasswordStore,PasswordStoreEntry};

use std::ops::Drop;
use std::cmp::{min,max};

struct TuiInterface {
	pw_selection_win: PasswordSelectionWindow,
	controls_win: ControlsWindow,
	pw_info_win: PasswordInfoWindow,
}

struct PasswordInfoWindow {
}

struct ControlsWindow {
}

struct PasswordSelectionWindow {
}

trait ClampCharLength {
	fn clamp_len<'a>(&'a self, len: usize) -> &'a str;
}

impl ClampCharLength for str {
	fn clamp_len<'a>(&'a self, len: usize) -> &'a str {
		if self.chars().count() <= len {self}
		else {&self[..len]}
	}
}

pub fn run_tui(password_store: &PasswordStore){
	//====== ncurses initialisation ======
	let ncurses = Ncurses::init();
	let Some(stdscr) = ncurses.stdscr()
	else {
		eprintln!("couldn't open stdscr");
		return;
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
	let mut selected_button = 0_isize; //for the control window
	//seperate so when the user clears the filter they go back to where they were
	let mut unfiltered_selected_password = 0_isize; 
	let mut filtered_selected_password = 0_isize;
	let mut filter = String::new();
	//====== update loop ======
	loop {
		//====== apply any password sorting or filters ======
		//let selectable_passwords = vec!["amazon.co.uk","ebay.co.uk","netflix.co.uk","gmail.com","web.whatsapp.com","outlook.com","disneyplus.com","hotmail.com","realy-long-domain-name.co.uk","nhs.gov.uk","uk.webuy.com","accounts.firefox.com","cad.onshape.com","homebase.co.uk","nowtv.com","my.integralmaths.org","computer password","phone password","totp recovery codes"]
		//	.into_iter()
		//	.map(|id| PasswordStoreEntry::new(id,"username1","password1","this password has notes"))
		//	.collect::<Vec<_>>();
		let selectable_passwords = password_store.entries();
		filtered_selected_password = min(filtered_selected_password,selectable_passwords.len() as isize);
		let selectable_passwords_index = 
			if filter.len() > 0 {filtered_selected_password}
			else {unfiltered_selected_password} as usize;
		//====== render each window ======
		render_selections(
			&selection_window,
			&selectable_passwords.iter().map(|p| p.identifier()).collect::<Vec<_>>(),
			selectable_passwords_index
		);
		if selectable_passwords.len() == 0 {
			//no password to show
			reset_window(&info_window);
			let (height,width) = info_window.getmaxyx();
			if height > 2 && width > 2 {
				info_window.mvaddstr(1,1,"no password selected".clamp_len(width-2));
			}
		}else {
			show_password_info(&info_window,&selectable_passwords[selectable_passwords_index]);
		}
		render_control_window(&control_window,selected_button as usize,&filter);
		selection_window.refresh();
		info_window.refresh();
		control_window.refresh();
		//====== input handling ======
		match ncurses.getch() {
			Input::Up => if filter.len() > 0 {filtered_selected_password-=1} else {unfiltered_selected_password-=1},
			Input::Down => if filter.len() > 0 {filtered_selected_password+=1} else {unfiltered_selected_password+=1},
			Input::Left => selected_button -= 1,
			Input::Right => selected_button += 1,
			Input::Resize => redraw_windows(&stdscr,&selection_window,&control_window,&info_window),
			Input::AsciiChar(ch) => filter.push(ch),
			Input::Backspace => {filter.pop();},
			Input::Enter => {
				show_selection_menu(&ncurses,"Sort by",["identifier","time added","username"]);
			},
			_ => ()
		}
		unfiltered_selected_password = unfiltered_selected_password.rem_euclid(
			//cant mod 0
			max(1,password_store.entries().len()) as isize
		);
		filtered_selected_password = filtered_selected_password.rem_euclid(
			//cant mod 0
			max(1,selectable_passwords.len()) as isize
		);
		//rem_euclid is mod that works for negative numbers
		selected_button = selected_button.rem_euclid(4);
	}
	//====== cleanup ======
	control_window.delwin();
	selection_window.delwin();
	info_window.delwin();
	stdscr.delwin();
	ncurses.end();
}

fn reset_window(window: &Window){
	let (height,width) = window.getmaxyx();
	if height < 2 || width < 2 {return}
	for y in 0..(height-2){
		//====== fill with spaces ======
		window.mvaddstr(y+1,1," ".repeat(width-2));
		//====== reset all character attributes ======
		window.mvchgat(y+1,1,(width-2) as isize,&[VideoAttribute::Normal],0);
	}
}

fn render_selections(selection_window: &Window, selections: &[String], selection_index: usize){
	//====== figure out where the cursor and all the selections should be ======
	let (height,width) = selection_window.getmaxyx();
	//dont do anything
	if selections.len() == 0 || height < 3 || width < 3 {return}
	//I cast... 1000000 isizes!!!!!
	let cursor_index = max(
		min(selection_index,(height-2)/2) as isize,
		max((selection_index as isize) - max(selections.len() as isize,height as isize - 2) + (height as isize - 2),0
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

fn show_selection_menu<T: IntoIterator<Item=impl AsRef<str>>>(ncurses: &Ncurses, title: &str, options: T) -> Option<usize> {
	let options = options
		.into_iter()
		.map(|x| x.as_ref().to_string())
		.collect::<Vec<String>>();
	None
}

fn render_control_window(control_window: &Window, selected_button: usize, filter: &str){
	let (height,width) = control_window.getmaxyx();
	if width < 11 || height < 4 {return}
	//====== render search bar ======
	control_window.mvaddstr(1,1,"search: ");
	let filter_input_length = width-2-8-1; //remove one for style
	let filter_input_offset = max(0,(filter.len() as isize) - (filter_input_length as isize)) as usize;
	let viewable_filter_chars = format!("{:1$}",&filter[filter_input_offset..],filter_input_length);
	control_window.mvaddstr(1,9,viewable_filter_chars);
	control_window.mvchgat(1,9,filter_input_length as isize,&[VideoAttribute::Underline],0);
	//====== render buttons ======
	if width < 27 || height < 4 {return}
	control_window.mvaddstr(2,1,"<new> <del> <edit> <sort>");
	//underline selected button
	match selected_button {
		0 => control_window.mvchgat(2,2,3,&[VideoAttribute::Reverse],0),
		1 => control_window.mvchgat(2,8,3,&[VideoAttribute::Reverse],0),
		2 => control_window.mvchgat(2,14,4,&[VideoAttribute::Reverse],0),
		3 => control_window.mvchgat(2,21,4,&[VideoAttribute::Reverse],0),
		_ => ()
	}
	//====== move cursor to search bar ======
	control_window.wmove(1,9+min(filter_input_length,filter.len()));
}

fn show_password_info(info_window: &Window, password_entry: &PasswordStoreEntry){
	let (height,width) = info_window.getmaxyx();
	if width <= 2 {return}
	//====== only show as much info as can fit in the window ======
	//identifier
	if height < 3 {return}
	info_window.mvaddstr(1,1,
		format!("{:1$}",password_entry.identifier().clamp_len(width-2),width-2)
	);
	//====== username ======
	if height < 4 {return}
	info_window.mvaddstr(2,1,
		format!("username: {:1$}",password_entry.username(),width-2).clamp_len(width-2)
	);
	//====== password ======
	if height < 5 {return}
	info_window.mvaddstr(3,1,
		format!("password: \"{}\"{}",password_entry.password()," ".repeat(width-2)).clamp_len(width-2)
	);
	//====== notes ======
	if height < 6 {return}
	info_window.mvaddstr(4,1,
		format!("notes: {:1$}",password_entry.notes(),width-2).clamp_len(width-2)
	);
	//====== date added ======
}

fn redraw_windows(stdscr: &Window, selection_window: &Window, control_window: &Window, info_window: &Window){
	//====== calculate window sizes and positions ======
	let (term_height,term_width) = stdscr.getmaxyx();
	let selection_window_width = term_width/3;
	let selection_window_height = term_height;
	let selection_window_x = 0;
	let selection_window_y = 0;

	let control_window_height = 4;
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
