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
}

impl Drop for Window<'_> {
	fn drop(&mut self){
		unsafe {delwin(self.as_ptr())};
	}
}
