use crate::ncurses::{Ncurses,Input,Window,VideoAttribute};
use crate::password_store::{PasswordStore,PasswordStoreEntry};

use std::ops::Drop;
use std::cmp::{min,max};
use std::mem::MaybeUninit;

struct TuiInterface<'a,'b> {
	ncurses: Ncurses,
	stdscr: Window<'a>,
	selection_window: Window<'a>,
	controls_window: Window<'a>,
	info_window: Window<'a>,
	selected_button: isize,
	filtered_selected_password: isize,
	unfiltered_selected_password: isize,
	filter: String,
	password_store: &'b PasswordStore,
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

impl TuiInterface<'_,'_> {
	fn redraw_all(&mut self){
		self.redraw_selection_window();
		self.redraw_controls_window();
		self.redraw_info_window();
	}
	
	fn update_all(&mut self){
		self.update_selection_window();
		self.update_info_window();
		//controls window updates cursor so needs to be last
		self.update_controls_window();
	}

	fn redraw_selection_window(&self){
		//====== calculate window size and position ======
		let (term_height,term_width) = self.stdscr.getmaxyx();
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
		let (term_height,term_width) = self.stdscr.getmaxyx();
		let controls_window_height = 4;
		let controls_window_width = term_width-term_width/3;
		let controls_window_x = term_width/3;
		let controls_window_y = term_height-controls_window_height;
		//====== erase current window ======
		self.controls_window.erase();
		//====== move and resize ======
		//resize
		self.controls_window.resize(controls_window_height,controls_window_width);
		//move
		self.controls_window.mvwin(controls_window_y,controls_window_x);
		//====== redraw borders ======
		self.controls_window.r#box(0,0);
		//refresh
		self.controls_window.refresh();
	}
	fn redraw_info_window(&self){
		//====== calculate window size and position ======
		let (term_height,term_width) = self.stdscr.getmaxyx();
		let info_window_width = term_width - term_width/3;
		let info_window_height = term_height-4;
		let info_window_x = term_width/3;
		let info_window_y = 0;
		//====== erase current window ======
		self.info_window.erase();
		//====== move and resize ======
		//resize
		self.info_window.resize(info_window_height,info_window_width);
		//move
		self.info_window.mvwin(info_window_y,info_window_x);
		//====== redraw borders ======
		self.info_window.r#box(0,0);
		//refresh
		self.info_window.refresh();
	}
	
	fn change_selected_password(&mut self, offset: isize){
		//cant select nothing
		if self.password_store.password_count() == 0 {return}
		//change filtered or unfiltered selected password
		if self.filter.len() > 0 {
			self.filtered_selected_password += offset;
		}else {
			let new_selection = self.unfiltered_selected_password + offset;
			self.unfiltered_selected_password = new_selection
				.rem_euclid(self.password_store.password_count() as isize);
		}
	}

	fn update_filter(&mut self, new_filter: String){
		self.filter = new_filter;
		self.filtered_selected_password = min(self.filtered_selected_password,self.filter_passwords().len() as isize);
	}

	fn filter_passwords(&self) -> Vec<PasswordStoreEntry> {
		self.password_store
			.entries()
			.into_iter()
			.filter(|p| self.filter.len() != 0 && p.identifier().find(&self.filter).is_some())
			.collect()
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
				.count()
		}else {
			self.unfiltered_selected_password as usize
		}
	}

	fn update_selection_window(&mut self){
		//====== apply filter ======
		let selections = self.filter_passwords()
			.into_iter()
			.map(|p| p.identifier())
			.collect::<Vec<_>>();
		self.filtered_selected_password = self.filtered_selected_password
			.rem_euclid(self.password_store.password_count() as isize);
		//====== show selection window ======
		let selection_index =
			if self.filter.len() != 0 {self.filtered_selected_password}
			else {self.unfiltered_selected_password};
		self.render_selections(&self.selection_window,selections.as_slice(),selection_index as usize);
		//====== refresh window ======
		self.selection_window.refresh();
	}

	fn update_controls_window(&self){
		let (height,width) = self.controls_window.getmaxyx();
		if width < 11 || height < 4 {return}
		//====== render search bar ======
		self.controls_window.mvaddstr(1,1,"search: ");
		let filter_input_length = width-2-8-1; //remove one for style
		let filter_input_offset = max(0,(self.filter.len() as isize) - (filter_input_length as isize)) as usize;
		let viewable_filter_chars = format!("{:1$}",&self.filter[filter_input_offset..],filter_input_length);
		self.controls_window.mvaddstr(1,9,viewable_filter_chars);
		self.controls_window.mvchgat(1,9,filter_input_length as isize,&[VideoAttribute::Underline],0);
		//====== render buttons ======
		if width < 27 || height < 4 {return}
		self.controls_window.mvaddstr(2,1,"<new> <del> <edit> <sort>");
		//underline selected button
		match self.selected_button {
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
		let (height,width) = self.info_window.getmaxyx();
		//====== check password available to show ======
		let passwords = self.password_store.entries();
		let Some(password_entry) = passwords
			.get(self.selected_password_index())
		else {
			//no password to show
			reset_window(&self.info_window);
			let (height,width) = self.info_window.getmaxyx();
			if height > 2 && width > 2 {
				self.info_window.mvaddstr(1,1,"no password selected".clamp_len(width-2));
			}
			self.info_window.refresh();
			return;
		};
		if width <= 2 {return}
		//====== only show as much info as can fit in the window ======
		//identifier
		if height < 3 {return}
		self.info_window.mvaddstr(1,1,
			format!("{:1$}",password_entry.identifier().clamp_len(width-2),width-2)
		);
		//====== username ======
		if height < 4 {return}
		self.info_window.mvaddstr(2,1,
			format!("username: {:1$}",password_entry.username(),width-2).clamp_len(width-2)
		);
		//====== password ======
		if height < 5 {return}
		self.info_window.mvaddstr(3,1,
			format!("password: \"{}\"{}",password_entry.password()," ".repeat(width-2)).clamp_len(width-2)
		);
		//====== notes ======
		if height < 6 {return}
		self.info_window.mvaddstr(4,1,
			format!("notes: {:1$}",password_entry.notes(),width-2).clamp_len(width-2)
		);
		//====== date added ======
		//TODO
	}

	//we can reuse this for popup selection windows and the main password selection window
	fn render_selections(&self, window: &Window, selections: &[String], selection_index: usize){
		//====== figure out where the cursor and all the selections should be ======
		let (height,width) = window.getmaxyx();
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
			window.mvaddstr(i+1,1,format!("{:1$}",truncated_selection,width-2,)); //pad with spaces
			//remove any previous highlighting
			window.mvchgat(i+1,1,(width-2) as isize,&[VideoAttribute::Normal],0);
		}
		//====== highlight the current selection ======
		window.mvchgat(cursor_index+1,1,(width-2) as isize,&[VideoAttribute::Reverse],0);
	}
}

pub fn run_tui(password_store: &PasswordStore){
	//====== create windows ======
	let mut tui_interface = TuiInterface {
		//variables
		selected_button: 0,
		unfiltered_selected_password: 0, //seperate so when the user clears the filter they go back to where they were
		filtered_selected_password: 0,
		filter: String::new(),
		password_store: password_store,
		//windows
		//TODO: this is UB but im lowkey to tired to fix rn
		ncurses: unsafe {MaybeUninit::<Ncurses>::zeroed().assume_init()},
		selection_window: unsafe {MaybeUninit::<Window>::zeroed().assume_init()},
		controls_window: unsafe {MaybeUninit::<Window>::zeroed().assume_init()},
		info_window: unsafe {MaybeUninit::<Window>::zeroed().assume_init()},
		stdscr: unsafe {MaybeUninit::<Window>::zeroed().assume_init()},
	};
	//====== ncurses initialisation ======
	let ncurses = Ncurses::init();
	//windows
	tui_interface.ncurses = ncurses;
	tui_interface.selection_window = tui_interface.ncurses.newwin(1,1,0,0);
	tui_interface.controls_window = tui_interface.ncurses.newwin(1,1,0,0);
	tui_interface.info_window = tui_interface.ncurses.newwin(1,1,0,0);
	//stdscr
	let Some(stdscr) = tui_interface.ncurses.stdscr()
	else {
		eprintln!("couldn't open stdscr");
		return;
	};
	tui_interface.stdscr = stdscr;
	//fiddle with input settings
	tui_interface.stdscr.keypad(true);
	tui_interface.ncurses.noecho();
	tui_interface.stdscr = stdscr;
	//update window sizes
	tui_interface.redraw_all();
	//====== update loop ======
	loop {
		//====== render each window ======
		tui_interface.update_all();
		//====== input handling ======
		match tui_interface.ncurses.getch() {
			Input::Up => tui_interface.change_selected_password(1),
			Input::Down => tui_interface.change_selected_password(-1),
			Input::Left => (),
			Input::Right => (),
			Input::Resize => tui_interface.redraw_all(),
			Input::AsciiChar(ch) => (),
			Input::Backspace => (),
			Input::Enter => {
				show_selection_menu(&tui_interface.ncurses,"Sort by",["identifier","time added","username"]);
			},
			_ => ()
		}
	}
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

