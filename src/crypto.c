#include <unistd.h>
#include <string.h>
#include <stdint.h>
#include <sys/socket.h>
#include <linux/if_alg.h>

#define MIN(a,b) (((a) < (b)) ? (a) : (b))
#define MAX(a,b) (((a) > (b)) ? (a) : (b))

#define MAX_IV_LEN 32

struct skcipher_info {
	const char *alg_name;

	const char *message;
	size_t message_len;

	const char *key;
	size_t key_len;

	const char *iv;
	size_t iv_len;
	
	char *output;
	size_t output_len;
};

//using the kernel's built in crypto api
//https://docs.kernel.org/crypto/userspace-if.html
int sha256(const char *message, size_t message_len, unsigned char *message_digest){
	//====== open a crypto socket ======
	//open a crypto connection for which we can later accept to create new sockets from
	int crypto_socket_provider = socket(AF_ALG,SOCK_SEQPACKET,0);
	if (crypto_socket_provider < 0) return -1;
	struct sockaddr_alg alg_info = {
		.salg_family = AF_ALG,
		.salg_type = "hash",
		.salg_name = "sha256",
	};
	int result = bind(crypto_socket_provider,(struct sockaddr *)&alg_info,sizeof(alg_info));
	if (result < 0) return -1;
	//accept a new socket that can be used to sha1 digest
	int crypto_socket = accept(crypto_socket_provider,NULL,0);
	if (crypto_socket < 0) return -1;
	//====== send data to be hashed ======
	result = send(crypto_socket,message,message_len,0);
	if (result < 0) return -1;
	//====== receive digest ======
	result = recv(crypto_socket,message_digest,32,0);
	if (result < 0) return -1;
	//====== close the socket ======
	close(crypto_socket);
	close(crypto_socket_provider);
	return 0;
}

int skcipher(struct skcipher_info *skcipher_info, int encrypt){
	//====== open a crypto socket ======
	//open a crypto connection for which we can later accept to create new sockets from
	int crypto_socket_provider = socket(AF_ALG,SOCK_SEQPACKET,0);
	if (crypto_socket_provider < 0) return -1;
	struct sockaddr_alg alg_info = {
		.salg_family = AF_ALG,
		.salg_type = "skcipher",
		.salg_name = "cbc(aes)",
	};
	int result = bind(crypto_socket_provider,(struct sockaddr *)&alg_info,sizeof(alg_info));
	if (result < 0) return -1;
	//====== set key ======
	result = setsockopt(crypto_socket_provider,SOL_ALG,ALG_SET_KEY,skcipher_info->key,skcipher_info->key_len);
	if (result != 0) return -1;
	//====== accept a new socket ======
	int crypto_socket = accept(crypto_socket_provider,NULL,0);
	if (crypto_socket < 0) return -1;

	//====== prepare msghdr for sendmsg ======
	size_t iv_len = MIN(MAX_IV_LEN,skcipher_info->iv_len);
	char cmsg_buf[CMSG_SPACE(sizeof(uint32_t)) + CMSG_SPACE(sizeof(struct af_alg_iv) + MAX_IV_LEN)] = {0};
	struct iovec iov[1] = {{ 
		.iov_base = skcipher_info->message,
		.iov_len = skcipher_info->message_len,
	}};
	struct msghdr message = {
		.msg_iov = iov,
		.msg_iovlen = 1,
		.msg_control = cmsg_buf,
		.msg_controllen = sizeof(cmsg_buf) - (MAX_IV_LEN - iv_len),
	};
	//====== set cmesg data ======
	//encryption or decryption
	struct cmsghdr *cmsg_item = CMSG_FIRSTHDR(&message);
	cmsg_item->cmsg_len = CMSG_LEN(sizeof(uint32_t));
	cmsg_item->cmsg_level = SOL_ALG;
	cmsg_item->cmsg_type = ALG_SET_OP;
	*((uint32_t *)CMSG_DATA(cmsg_item)) = (encrypt == 1) ? ALG_OP_ENCRYPT : ALG_OP_DECRYPT;
	//iv
	cmsg_item = CMSG_NXTHDR(&message,cmsg_item);
	cmsg_item->cmsg_len = CMSG_LEN(sizeof(struct af_alg_iv) + iv_len);
	cmsg_item->cmsg_level = SOL_ALG;
	cmsg_item->cmsg_type = ALG_SET_IV;
	struct af_alg_iv *iv = (void *)CMSG_DATA(cmsg_item);
	iv->ivlen = iv_len;
	memcpy(&iv->iv,skcipher_info->iv,iv_len);
	//====== sendmsg the data ======
	result = sendmsg(crypto_socket,&message,0);
	if (result < 0) return -1;
	//====== receive digest ======
	result = recv(crypto_socket,skcipher_info->output,skcipher_info->output_len,0);
	if (result < 0) return -1;
	//====== close the socket ======
	close(crypto_socket);
	close(crypto_socket_provider);
	return 0;
}
