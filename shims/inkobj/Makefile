CC = x86_64-w64-mingw32-gcc
CFLAGS = -O2 -Wall -Wextra -shared -s
LDFLAGS = -lole32 -luuid

all: ms365ink.dll

ms365ink.dll: ms365ink.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS)

clean:
	rm -f ms365ink.dll
