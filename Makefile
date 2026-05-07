CFLAGS=-Wall -Wextra -g
LFLAGS=
RC=rustc

passwdmgr : src/main.rs src/password_store.rs src/crypto.rs src/args.rs src/term.o src/crypto.o
	$(RC) -o $@ src/main.rs $(LFLAGS) -Clink-arg=src/term.o -Clink-arg=src/crypto.o --edition 2024

crypto-test : src/crypto_test.o
	$(CC) -o $@ $^ -fsanitize=address
	
