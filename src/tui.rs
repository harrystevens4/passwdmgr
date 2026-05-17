use crate::ncurses::{Ncurses,Input,Window,VideoAttribute};
use crate::password_store::{PasswordStore,PasswordStoreEntry};

use std::ops::Drop;
use std::cmp::{min,max};

struct TuiInterface<'_> {
	ncurses: Ncurses,
	stdscr: Window<'_>,
	selection_window: Window<'_>,
	controls_window: Window<'_>,
	info_window: Window<'_>,
	selected_button: usize,
	filtered_selected_password: usize,
	unfiltered_selected_password: usize,
	filter: String,
	password_store: PasswordStore,
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

impl TuiInterface {
	fn redraw_all(&mut self){
		self.redraw_selection_window();
		self.redraw_controls_window();
		self.redraw_info();
	}
	
	fn update_all(&mut self){
		self.update_selection_window();
		self.update_info_window();
		//controls window updates cursor so needs to be last
		self.update_controls_window();
	}

	fn redraw_selection_window(&self){
		//====== calculate window size and position ======
		let (term_height,term_width) = stdscr.getmaxyx();
		let selection_window_width = term_width/3;
		let selection_window_height = term_height;
		let selection_window_x = 0;
		let selection_window_y = 0;
		//====== erase current window ======
		self.selection_window.erase();
		//====== move and resize ======
		//resize
		self.selection_window.resize(selection_window_height,selection_window_width);
		//move
		self.selection_window.mvwin(selection_window_y,selection_window_x);
		//====== redraw borders ======
		self.selection_window.r#box(0,0);
		//refresh
		self.selection_window.refresh();
	}
	fn redraw_controls_window(&self){
		//====== calculate window size and position ======
		let (term_height,term_width) = stdscr.getmaxyx();
		let control_window_height = 4;
		let control_window_width = term_width-selection_window_width;
		let control_window_x = selection_window_width;
		let control_window_y = term_height-control_window_height;
		//====== erase current window ======
		self.control_window.erase();
		//====== move and resize ======
		//resize
		self.control_window.resize(control_window_height,control_window_width);
		//move
		self.control_window.mvwin(control_window_y,control_window_x);
		//====== redraw borders ======
		self.control_window.r#box(0,0);
		//refresh
		self.control_window.refresh();
	}
	fn redraw_info_window(&self){
		//====== calculate window size and position ======
		let (term_height,term_width) = stdscr.getmaxyx();
		let info_window_width = control_window_width;
		let info_window_height = term_height-control_window_height;
		let info_window_x = selection_window_width;
		let info_window_y = 0;
		//====== erase current window ======
		info_window.erase();
		//====== move and resize ======
		//resize
		info_window.resize(info_window_height,info_window_width);
		//move
		info_window.mvwin(info_window_y,info_window_x);
		//====== redraw borders ======
		info_window.r#box(0,0);
		//refresh
		info_window.refresh();
	}
	
	fn change_selected_password(&mut self, offset: isize){
		//cant select nothing
		if self.password_store.password_count() == 0 {return}
		//change filtered or unfiltered selected password
		if self.filter.len() > 0 {
			filtered_selected_password += offset;
		}else {
			unfiltered_selected_password += offset;
			unfiltered_selected_password = unfiltered_selected_password
				.rem_euclid(self.password_store.password_count());
		}
	}

	fn update_filter(&mut self){
		self.filtered_selected_password = min(self.filtered_selected_password,selectable_passwords.len() as isize);
	}

	fn filter_passwords() -> Vec<PasswordStoreEntry> {
		self.password_store
			.entries()
			.into_iter()
			.filter(|p| if self.filter.len() != 0 && p.identifier().find(&self.filter).is_some())
			.collect<Vec<String>>();
	}

	fn selected_password_index(&self) -> usize {
		if self.filter.len() != 0 {
			self.password_store
				.entries()
				.into_iter()
				.scan(self.filtered_selected_password,|i,password|{
					if password.identifier().find(&self.filter).is_some() {
						*i -= 1
					}
					if *i == 0 {None}
					else {Some(password)}
				})
				.count();
		}else {
			self.unfiltered_selected_password
		}
	}

	fn update_selection_window(&mut self){
		//====== apply filter ======
		let selections = self.filter_passwords();
		filtered_selected_password = filtered_selected_password
			.rem_euclid(self.password_store.password_count());
		//====== show selection window ======
		let selection_index =
			if self.filter.len() != 0 {self.filtered_selected_password}
			else {self.unfiltered_selected_password};
		self.render_selections(self.selection_window,&selections,selection_index);
		//====== refresh window ======
		self.selection_window.refresh();
	}

	fn update_controls_window(&self){
		let (height,width) = self.controls_window.getmaxyx();
		if width < 11 || height < 4 {return}
		//====== render search bar ======
		control_window.mvaddstr(1,1,"search: ");
		let filter_input_length = width-2-8-1; //remove one for style
		let filter_input_offset = max(0,(self.filter.len() as isize) - (filter_input_length as isize)) as usize;
		let viewable_filter_chars = format!("{:1$}",&filter[filter_input_offset..],filter_input_length);
		self.control_window.mvaddstr(1,9,viewable_filter_chars);
		self.control_window.mvchgat(1,9,filter_input_length as isize,&[VideoAttribute::Underline],0);
		//====== render buttons ======
		if width < 27 || height < 4 {return}
		self.controls_window.mvaddstr(2,1,"<new> <del> <edit> <sort>");
		//underline selected button
		match selected_button {
			0 => self.controls_window.mvchgat(2,2,3,&[VideoAttribute::Reverse],0),
			1 => self.controls_window.mvchgat(2,8,3,&[VideoAttribute::Reverse],0),
			2 => self.controls_window.mvchgat(2,14,4,&[VideoAttribute::Reverse],0),
			3 => self.controls_window.mvchgat(2,21,4,&[VideoAttribute::Reverse],0),
			_ => ()
		}
		//====== move cursor to search bar ======
		self.controls_window.wmove(1,9+min(filter_input_length,self.filter.len()));
		//====== refresh window ======
		self.selection_window.refresh();
	}

	fn update_info_window(&self){
		let Some(password_entry) = self.password_store
			.entries()
			.get(self.selected_password_index())
		else {return};
		let (height,width) = self.info_window.getmaxyx();
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
		//TODO
	}

	//we can reuse this for popup selection windows and the main password selection window
	fn render_selections(&self, window: &Window, selections: &[String], selection_index: usize){
		//====== figure out where the cursor and all the selections should be ======
		let (height,width) = self.selection_window.getmaxyx();
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
	
	fn update_controls_window(&self){
		self.controls_window.refresh();
	}
	
	fn update_info_window(&self){
		self.info_window.refresh();
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
	//====== create windows ======
	let mut tui_interface = TuiInterface {
		//windows
		ncurses,
		stdscr,
		selection_window: ncurses.newwin(1,1,0,0),
		control_window: ncurses.newwin(1,1,0,0),
		info_window: ncurses.newwin(1,1,0,0),
		//variables
		selected_button: 0,
		unfiltered_selected_password: 0, //seperate so when the user clears the filter they go back to where they were
		filtered_selected_password: 0,
		filter: String::new(),
	}
	//update window sizes
	tui_interface.redraw_windows();
	//====== update loop ======
	loop {
		let selectable_passwords = password_store.entries();
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
		//====== input handling ======
		match ncurses.getch() {
			Input::Up => tui_interface.change_selected_password(1),
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


fn show_selection_menu<T: IntoIterator<Item=impl AsRef<str>>>(ncurses: &Ncurses, title: &str, options: T) -> Option<usize> {
	let options = options
		.into_iter()
		.map(|x| x.as_ref().to_string())
		.collect::<Vec<String>>();
	None
}

