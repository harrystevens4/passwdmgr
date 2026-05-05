CFLAGS=-Wall -Wextra -g
LFLAGS=
RC=rustc

passwdmgr : src/main.rs src/password_store.rs src/crypto.rs src/args.rs src/term.o
	$(RC) -o $@ src/main.rs $(LFLAGS) -Clink-arg=src/term.o
	
