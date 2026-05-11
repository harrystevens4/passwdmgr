use std::ffi::*;

pub type chtype = c_uint;
pub type WindowPtr = *mut c_void;

unsafe extern "C" {
	fn initscr() -> WindowPtr;
	fn endwin() -> WindowPtr;
	fn wrefresh(window: WindowPtr) -> c_int;
	fn wborder(win: WindowPtr, ls: chtype, rs: chtype, ts: chtype, bs: chtype, tl: chtype, tr: chtype, bl: chtype, br: chtype) -> c_int;
	fn newwin(nlines: c_int, ncols: c_int, begin_y: c_int, begin_x: c_int) -> WindowPtr;
	fn delwin(win: WindowPtr) -> c_int;
	fn getmaxx(win: WindowPtr) -> c_int;
	fn getmaxy(win: WindowPtr) -> c_int;
	fn mvwaddnwstr(win: WindowPtr, y: c_int, x: c_int, wstr: *const c_void, n: c_int) -> c_int;
	fn mbstowcs(dest: *mut c_void, src: *const c_char, dsize: c_size_t) -> c_size_t;
	fn getch() -> c_int;
}

pub struct Ncurses {
	stdscr: WindowPtr,
}

#[derive(Clone)]
pub struct Window<'a> {
	ptr: WindowPtr,
	__ncurses: &'a Ncurses, //leverage the borrow checker to ensure all windows are destroyed before ncurses is deinitialised at compile time!
}

impl Ncurses {
	pub fn init() -> Self {
		let stdscr = unsafe {initscr()};
		unsafe {wrefresh(stdscr)};
		Self {
			stdscr,
		}
	}
	pub fn stdscr(&self) -> Window<'_> {
		Window {
			ptr: self.stdscr,
			__ncurses: &self,
		}
	}
	pub fn end(self) {} //drop self
	pub fn newwin(&self, lines: usize, cols: usize, begin_y: usize, begin_x: usize) -> Window<'_> {
		let window = unsafe {newwin(lines as c_int,cols as c_int,begin_y as c_int,begin_x as c_int)};
		Window {
			ptr: window,
			__ncurses: &self
		}
	}
}

impl Drop for Ncurses {
	fn drop(&mut self){
		unsafe {endwin()};
	}
}

impl Window<'_> {
	pub fn as_ptr(&self) -> WindowPtr {self.ptr}
	pub fn refresh(&self) {unsafe {wrefresh(self.as_ptr())};}
	pub fn r#box(&self, verch: chtype, horch: chtype) {unsafe {wborder(self.as_ptr(),verch,horch,0,0,0,0,0,0)};}
	pub fn getmaxyx(&self) -> (usize,usize) { //(y,x)
		let x = unsafe {getmaxx(self.as_ptr())};
		let y = unsafe {getmaxy(self.as_ptr())};
		(y as usize,x as usize)
	}
	pub fn delwin(self) {} //drop self
	pub fn mvaddstr<T: AsRef<str>>(&self, y: usize, x: usize, string: T){
		let string = string.as_ref();
		let Ok(cstring) = CString::new(string)
		else {return};
		//convert to wchar_t[]
		let mut wchar_buffer: Vec<u64> = vec![0; string.chars().count()];
		let wchar_buffer_ptr = wchar_buffer.as_mut_ptr();
		unsafe {mbstowcs(
			wchar_buffer_ptr as *mut c_void,
			cstring.as_ptr() as *const i8,
			string.chars().count()
		)};
		//use wide chars by default
		unsafe {mvwaddnwstr(
			self.as_ptr(),
			y as c_int,
			x as c_int,
			wchar_buffer_ptr as *mut c_void,
			string.chars().count() as c_int
		)};
	}
}

impl Drop for Window<'_> {
	fn drop(&mut self){
		unsafe {delwin(self.as_ptr())};
	}
}
