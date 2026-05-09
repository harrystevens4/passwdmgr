use std::ffi::*;
use std::ptr;

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
	stdscr: Window,
	windows: Vec<Window>,
}

#[derive(Clone,Copy)]
pub struct Window(WindowPtr);

impl From<WindowPtr> for Window {
	fn from(v: WindowPtr) -> Window {
		Window(v)
	}
}

impl From<Window> for WindowPtr {
	fn from(v: Window) -> WindowPtr {
		let Window(ptr) = v;
		ptr
	}
}

impl Ncurses {
	pub fn init() -> Self {
		let stdscr = unsafe {initscr()};
		unsafe {wrefresh(stdscr)};
		Self {
			stdscr: Window(stdscr),
			windows: vec![],
		}
	}
	pub fn stdscr(&self) -> Window {self.stdscr}
	pub fn end(self) {unsafe {endwin()};}
	pub fn newwin(&mut self, lines: usize, cols: usize, begin_y: usize, begin_x: usize) -> Window {
		let window = unsafe {newwin(lines as c_int,cols as c_int,begin_y as c_int,begin_x as c_int)};
		self.windows.push(window.into());
		window.into()
	}
	pub fn delwin(&mut self, window: Window){
		for i in 0..(self.windows.len()) {
			if ptr::eq(window.as_ptr(),self.windows[i].as_ptr()) {
				self.windows.remove(i);
				return;
			}
		}
	}
}

impl Drop for Ncurses {
	fn drop(&mut self){
		for window in self.windows.clone() {
			self.delwin(window);
		}
		unsafe {endwin()};
	}
}

impl Window {
	pub fn as_ptr(&self) -> WindowPtr {(*self).into()}
	pub fn refresh(&self) {unsafe {wrefresh((*self).into())};}
	pub fn r#box(&self, verch: chtype, horch: chtype) {unsafe {wborder((*self).into(),verch,horch,0,0,0,0,0,0)};}
	pub fn getmaxyx(&self) -> (usize,usize) { //(y,x)
		let x = unsafe {getmaxx(self.as_ptr())};
		let y = unsafe {getmaxy(self.as_ptr())};
		(y as usize,x as usize)
	}
}
