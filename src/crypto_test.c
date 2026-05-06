#include <sys/random.h>
#include <stdint.h>
#include <string.h>
#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <linux/if_alg.h>
#include <sys/socket.h>

#define MIN(a,b) (((a) < (b)) ? (a) : (b))
#define MAX(a,b) (((a) > (b)) ? (a) : (b))

#define IV_LEN 16

struct aes_data {
	const char *message;
	size_t message_len;

	const char *key;
	size_t key_len;

	char iv[IV_LEN];
	
	char *output;
	size_t output_len;
};

int aes(struct aes_data *aes_info, int encrypt){
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
	result = setsockopt(crypto_socket_provider,SOL_ALG,ALG_SET_KEY,aes_info->key,aes_info->key_len);
	if (result != 0) return -1;
	//====== accept a new socket ======
	int crypto_socket = accept(crypto_socket_provider,NULL,0);
	if (crypto_socket < 0) return -1;

	//====== send data to be encrypted ======
	char cmsg_buf[CMSG_SPACE(sizeof(uint32_t)) + CMSG_SPACE(sizeof(struct af_alg_iv) + IV_LEN)] = {0};
	struct iovec iov[1] = {{ 
		.iov_base = aes_info->message,
		.iov_len = aes_info->message_len,
	}};
	struct msghdr message = {
		.msg_iov = iov,
		.msg_iovlen = 1,
		.msg_control = cmsg_buf,
		.msg_controllen = sizeof(cmsg_buf)
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
	cmsg_item->cmsg_len = CMSG_LEN(sizeof(struct af_alg_iv) + IV_LEN);
	cmsg_item->cmsg_level = SOL_ALG;
	cmsg_item->cmsg_type = ALG_SET_IV;
	struct af_alg_iv *iv = (void *)CMSG_DATA(cmsg_item);
	iv->ivlen = IV_LEN;
	memcpy(&iv->iv,&aes_info->iv,IV_LEN);
	//====== sendmsg the data ======
	result = sendmsg(crypto_socket,&message,0);
	if (result < 0) return -1;
	//====== receive digest ======
	result = recv(crypto_socket,aes_info->output,aes_info->output_len,0);
	if (result < 0) return -1;
	//====== close the socket ======
	close(crypto_socket);
	close(crypto_socket_provider);
	return 0;
}

int main(int argc, char **argv){
	char data[1024*16] = {00};
	char output_1[1024*16] = {0};
	char output_2[1024*16] = {0};
	char key[16] = {0};
	char iv[16] = {0};
	//getrandom(key,sizeof(key),0);
	//getrandom(data,sizeof(data),0);
	//getrandom(iv,sizeof(iv),0);
	struct aes_data aes_info = {
		.message = data,
		.message_len = sizeof(data),
		.key = key,
		.key_len = sizeof(key),
		.output = output_1,
		.output_len = sizeof(output_1),
	};
	//====== encrypt ======
	int result = aes(&aes_info,1);
	if (result < 0){
		perror("aes");
		return 1;
	}
	printf("input:\n");
	for (size_t i = 0; i < aes_info.message_len; i++) printf("%.2x",aes_info.message[i]);
	printf("\n");
	printf("encrypted:\n");
	for (size_t i = 0; i < aes_info.output_len; i++) printf("%.2x",aes_info.output[i]);
	printf("\n");
	//====== decrypt ======
	aes_info.message = output_1;
	aes_info.message_len = sizeof(output_1);
	aes_info.output = output_2;
	aes_info.output_len = sizeof(output_2);
	result = aes(&aes_info,0);
	if (result < 0){
		perror("aes");
		return 1;
	}
	printf("decrypted:\n");
	for (size_t i = 0; i < aes_info.output_len; i++) printf("%.2x",aes_info.output[i]);
	printf("\n");
	return 0;
}
