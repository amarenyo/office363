CC := x86_64-w64-mingw32-gcc
OUT := ../../src-tauri/resources/sppc.dll

all: $(OUT)

$(OUT): sppc.c
	mkdir -p $(dir $(OUT))
	$(CC) -shared -O2 sppc.c -o $(OUT) -s

clean:
	rm -f $(OUT)
