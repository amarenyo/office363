/* x64 regression for OneNote's SOAP envelope serialization.
 * Build: x86_64-w64-mingw32-gcc xmllite_namespace.c -o xmllite_namespace.exe -lole32 -luuid
 * Run with WINEDLLOVERRIDES=xmllite=b to reproduce, xmllite=n to verify the fix.
 */
#define COBJMACROS
#include <windows.h>
#include <objidl.h>
#include <xmllite.h>
#include <stdio.h>
#include <string.h>
static const GUID writer_iid = {0x7279fc88,0x709d,0x4095,{0xb6,0x3d,0x69,0xfe,0x4b,0x0d,0x90,0x30}};
#define CHECK(call) do { HRESULT hr = (call); if (FAILED(hr)) { printf("FAIL: %s: %08lx\n", #call, (unsigned long)hr); return 2; } } while (0)
int main(void)
{
    typedef HRESULT (WINAPI *create_fn)(REFIID,void **,IMalloc *);
    HMODULE dll = LoadLibraryA("xmllite.dll");
    create_fn create = dll ? (create_fn)GetProcAddress(dll,"CreateXmlWriter") : NULL;
    IXmlWriter *writer;
    IStream *stream;
    HGLOBAL memory;
    STATSTG stat;
    const char *data, *first, *second;
    const WCHAR *uri = L"http://schemas.xmlsoap.org/soap/envelope/";
    if (!create) { printf("FAIL: XmlLite load: %lu\n", GetLastError()); return 2; }
    CHECK(CreateStreamOnHGlobal(NULL,TRUE,&stream));
    CHECK(create(&writer_iid,(void **)&writer,NULL));
    CHECK(IXmlWriter_SetOutput(writer,(IUnknown *)stream));
    CHECK(IXmlWriter_WriteStartElement(writer,L"s",L"Envelope",uri));
    CHECK(IXmlWriter_WriteAttributeString(writer,L"xmlns",L"s",NULL,uri));
    CHECK(IXmlWriter_WriteEndElement(writer));
    CHECK(IXmlWriter_Flush(writer));
    CHECK(IStream_Stat(stream,&stat,STATFLAG_NONAME));
    CHECK(GetHGlobalFromStream(stream,&memory));
    data = GlobalLock(memory);
    /* The stream is not NUL-terminated; make a bounded copy before searching. */
    char *xml = calloc((size_t)stat.cbSize.QuadPart + 1, 1);
    if (!xml || !data) return 2;
    memcpy(xml,data,(size_t)stat.cbSize.QuadPart);
    first = strstr(xml,"xmlns:s=");
    second = first ? strstr(first+1,"xmlns:s=") : NULL;
    printf("%s: %s\n",first && !second ? "PASS" : "FAIL",xml);
    int result = first && !second ? 0 : 1;
    free(xml);
    GlobalUnlock(memory);
    IXmlWriter_Release(writer);
    IStream_Release(stream);
    FreeLibrary(dll);
    return result;
}
