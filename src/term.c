#include <stdint.h>
#include <termios.h>
#include <unistd.h>

int tty_set_echo(int fd, int state){
	struct termios settings;
	int result = tcgetattr(fd,&settings);
	if (result != 0) return -1;
	if (state == 0){
		settings.c_lflag &= ~(ECHO);
	}else{
		settings.c_lflag |= (ECHO);
	}
	result = tcsetattr(fd,TCSANOW,&settings);
	if (result != 0) return -1;
	return 0;
}
