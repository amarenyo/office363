CC := x86_64-w64-mingw32-gcc
OUT := ../../src-tauri/resources/ole32.dll

all: $(OUT)

$(OUT): shim.c words_segmenter.h ole32_ge.def
	mkdir -p $(dir $(OUT))
	$(CC) -shared -O2 shim.c ole32_ge.def -o $(OUT) -lmsi -s

clean:
	rm -f $(OUT)
