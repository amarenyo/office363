/* Regression: a missing optional buffer must not be dereferenced, and the
 * subsequent WinHttpWriteData body must retain its declared Content-Length.
 * Build: x86_64-w64-mingw32-gcc winhttp_streamed_body.c -o winhttp_streamed_body.exe -lwinhttp
 * Run against a local HTTP server that checks POST body "office363-probe".
 */
#include <windows.h>
#include <winhttp.h>
#include <stdio.h>
#include <stdlib.h>
#define CHECK(call) do { if (!(call)) { printf("FAIL: %s: %lu\n",#call,GetLastError()); return 1; } } while (0)
int main(int argc, char **argv)
{
    const char body[] = "office363-probe";
    DWORD size = sizeof(body)-1, written = 0, status = 0, status_size = sizeof(status);
    if (argc != 2) return 2;
    /* Install Office363's import wrappers in this test process. */
    CHECK(LoadLibraryA("ole32.dll"));
    HINTERNET session = WinHttpOpen(L"Office363 regression",WINHTTP_ACCESS_TYPE_NO_PROXY,NULL,NULL,0);
    CHECK(session);
    CHECK(WinHttpSetTimeouts(session,3000,3000,3000,3000));
    HINTERNET connection = WinHttpConnect(session,L"127.0.0.1",(INTERNET_PORT)atoi(argv[1]),0);
    CHECK(connection);
    HINTERNET request = WinHttpOpenRequest(connection,L"POST",L"/probe",NULL,NULL,NULL,0);
    CHECK(request);
    CHECK(WinHttpSendRequest(request,NULL,0,NULL,size,size,0));
    CHECK(WinHttpWriteData(request,body,size,&written));
    CHECK(written == size);
    CHECK(WinHttpReceiveResponse(request,NULL));
    CHECK(WinHttpQueryHeaders(request,WINHTTP_QUERY_STATUS_CODE|WINHTTP_QUERY_FLAG_NUMBER,NULL,&status,&status_size,NULL));
    WinHttpCloseHandle(request);
    WinHttpCloseHandle(connection);
    WinHttpCloseHandle(session);
    printf("%s: streamed POST returned HTTP %lu\n",status==200 ? "PASS" : "FAIL",status);
    return status==200 ? 0 : 1;
}
