
# Interface

```
+----------------------------------+
| a@b.com                | a@b.com |
| password: abcdef       | c@d.com |
| notes: no capitals     | d@e.com |
| added: 11/12/2025      |         |
|                        |         |
+----------------------------------+
| <sort> <new> <delete> <update>   |
+----------------------------------+
```

# Password Store Layout

```
+-----------------+
| header          |
+-----------------+
|                 |
|                 |
| encrypted block |
|                 |
|                 |
+-----------------+

struct header {
	uint32_t magic_number;
	uint8_t encryption_algorithm;
	uint64_t encrypted_size; //encryption block
	uint64_t decrypted_size; //encryption block
	uint64_t password_entry_count;
}
struct password_entry {
	uint32_t size;
	char account[];
	char password[];
	char notes[];
}
struct password_store {
	struct header header;
	struct password_entry[];
}
```
