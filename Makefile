CFLAGS=-Wall -Wextra -g
LFLAGS=-lncursesw
RUSTFLAGS=-g
RC=rustc

passwdmgr : src/main.rs src/password_store.rs src/crypto.rs src/args.rs src/term.o src/crypto.o src/ncurses.rs src/constants.rs
	$(RC) -o $@ $(RUSTFLAGS) src/main.rs $(LFLAGS) -Clink-arg=src/term.o -Clink-arg=src/crypto.o -Clink-arg=src/constants.o --edition 2024

src/constants.rs : src/generate_constants.o
	$(CC) -o constant-generator $^ 
	./constant-generator > src/constants.rs

crypto-test : src/crypto_test.o
	$(CC) -o $@ $^ -fsanitize=address
	
