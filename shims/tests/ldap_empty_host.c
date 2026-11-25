/* x64 regression for wldap32's zero-host-list underflow. No network connection.
 * Build: x86_64-w64-mingw32-gcc ldap_empty_host.c -o ldap_empty_host.exe
 * Run in an Office363 prefix with the current shim installed.
 */
#include <windows.h>
#include <stdio.h>
typedef void *(__cdecl *InitW)(WCHAR *,ULONG);
typedef void *(__cdecl *InitA)(char *,ULONG);
typedef ULONG (__cdecl *Unbind)(void *);
int main(void)
{
    if(!LoadLibraryA("ole32.dll"))return 2;
    HMODULE m=LoadLibraryA("wldap32.dll");if(!m)return 2;
    InitW wide=(void *)GetProcAddress(m,"ldap_initW"),ordinal=(void *)GetProcAddress(m,(LPCSTR)145);
    InitA ansi=(void *)GetProcAddress(m,(LPCSTR)144);Unbind unbind=(void *)GetProcAddress(m,"ldap_unbind");
    if(!wide||!ansi||!unbind||wide!=ordinal)return 2;
    WCHAR *hosts[]={NULL,L"",L" \t\r\n",L"localhost"};
    for(int i=0;i<1000;i++){
        void *ld=ordinal(hosts[i%4],i%2?389:3268);
        if(!ld){printf("FAIL: W initialization %d\n",i);return 1;}unbind(ld);
        ld=ansi(i%2?"":" \t",389);
        if(!ld){printf("FAIL: A initialization %d\n",i);return 1;}unbind(ld);
    }
    puts("PASS: 2000 LDAP session initializations, empty/default/named hosts and ordinal imports");return 0;
}
