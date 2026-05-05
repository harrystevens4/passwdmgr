CFLAGS=-Wall -Wextra -g
LFLAGS=
RC=rustc

passwdmgr : src/main.rs src/password_store.rs src/crypto.rs src/args.rs
	$(RC) -o $@ src/main.rs $(LFLAGS)

