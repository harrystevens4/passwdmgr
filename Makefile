CFLAGS=-Wall -Wextra -g
LFLAGS=
RC=rustc

passwdmgr : src/main.rs
	$(RC) -o $@ $^ $(LFLAGS)

