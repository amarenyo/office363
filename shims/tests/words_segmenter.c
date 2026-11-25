/* x64 WinRT regression: Unicode boundaries, offsets, collections and lifetime.
 * Build: x86_64-w64-mingw32-gcc words_segmenter.c -o words_segmenter.exe
 * Run in an Office363 prefix with the updated ole32 shim installed.
 */
#include <windows.h>
#include <stdio.h>
typedef void *HS;
typedef struct { void **vt; } Obj;
#define CALL(o,n,type) ((type)((Obj *)(o))->vt[n])
#define CHECK(x) do { if (!(x)) { printf("FAIL line %d: %s\n",__LINE__,#x); return 1; } } while(0)
static const GUID factory_iid={0xe6977274,0xfc35,0x455c,{0x8b,0xfb,0x6d,0x7f,0x46,0x53,0xca,0x97}};
static const GUID iterable_iid={0xd14dc94a,0xa311,0x5ddc,{0xbf,0x8b,0xd5,0x87,0x95,0xd8,0x7d,0x6d}};
static const GUID word_iid={0xd2d4ba6d,0x987c,0x4cc0,{0xb6,0xbd,0xd4,0x9a,0x11,0xb3,0x8f,0x9a}};
static const GUID alternate_iterable_iid={0xae838c29,0x60c1,0x5093,{0xae,0x61,0x0e,0xd2,0x28,0x57,0xdb,0x05}};
typedef HRESULT (WINAPI *CreateString)(const WCHAR *,UINT32,HS *);
typedef HRESULT (WINAPI *Activate)(HS,REFIID,void **);
typedef const WCHAR *(WINAPI *Raw)(HS,UINT32 *);
typedef HRESULT (WINAPI *Delete)(HS);
typedef ULONG (WINAPI *Release)(void *);
static void release(void *o){CALL(o,2,Release)(o);}
int main(void)
{
    CHECK(LoadLibraryA("ole32.dll"));HMODULE cb=LoadLibraryA("combase.dll");CHECK(cb);
    CreateString create=(void *)GetProcAddress(cb,"WindowsCreateString");
    Delete del=(void *)GetProcAddress(cb,"WindowsDeleteString");Raw raw=(void *)GetProcAddress(cb,"WindowsGetStringRawBuffer");
    Activate activate=(void *)GetProcAddress(cb,"RoGetActivationFactory");CHECK(create&&del&&raw&&activate);
    HS cls=NULL,lang=NULL,text=NULL;void *factory=NULL,*seg=NULL,*view=NULL;
    const WCHAR input[]=L"\U0001f600 Gr\u00fcsse, Cafe\u0301! 123.";
    CHECK(SUCCEEDED(create(L"Windows.Data.Text.WordsSegmenter",lstrlenW(L"Windows.Data.Text.WordsSegmenter"),&cls)));
    CHECK(SUCCEEDED(create(L"de-CH",5,&lang)));
    CHECK(SUCCEEDED(create(input,lstrlenW(input),&text)));
    CHECK(SUCCEEDED(activate(cls,&factory_iid,&factory)));
    CHECK(SUCCEEDED(CALL(factory,6,HRESULT(WINAPI *)(void *,HS,void **))(factory,lang,&seg)));
    release(factory);del(lang);del(cls);
    CHECK(SUCCEEDED(CALL(seg,8,HRESULT(WINAPI *)(void *,HS,void **))(seg,text,&view)));
    UINT32 count=0;CHECK(SUCCEEDED(CALL(view,7,HRESULT(WINAPI *)(void *,UINT32 *))(view,&count))&&count==3);
    const UINT32 starts[]={3,11,18},lengths[]={6,5,3};
    for(UINT32 i=0;i<count;i++){
        void *word=NULL,*qi=NULL;UINT32 span[2]={0};HS wordtext=NULL;UINT32 len=0;
        CHECK(SUCCEEDED(CALL(view,6,HRESULT(WINAPI *)(void *,UINT32,void **))(view,i,&word)));
        CHECK(SUCCEEDED(CALL(word,0,HRESULT(WINAPI *)(void *,REFIID,void **))(word,&word_iid,&qi)));release(qi);
        CHECK(SUCCEEDED(CALL(word,7,HRESULT(WINAPI *)(void *,UINT32 *))(word,span)));
        CHECK(span[0]==starts[i]&&span[1]==lengths[i]);
        CHECK(SUCCEEDED(CALL(word,6,HRESULT(WINAPI *)(void *,HS *))(word,&wordtext)));
        const WCHAR *str=raw(wordtext,&len);CHECK(len==lengths[i]&&!memcmp(str,input+starts[i],len*sizeof(WCHAR)));
        void *alternates=NULL,*alt_iterable=NULL,*alt_iterator=NULL;BOOLEAN alt_has=TRUE;
        CHECK(SUCCEEDED(CALL(word,8,HRESULT(WINAPI *)(void *,void **))(word,&alternates)));
        CHECK(SUCCEEDED(CALL(alternates,0,HRESULT(WINAPI *)(void *,REFIID,void **))(alternates,&alternate_iterable_iid,&alt_iterable)));
        CHECK(SUCCEEDED(CALL(alt_iterable,6,HRESULT(WINAPI *)(void *,void **))(alt_iterable,&alt_iterator)));
        CHECK(SUCCEEDED(CALL(alt_iterator,7,HRESULT(WINAPI *)(void *,BOOLEAN *))(alt_iterator,&alt_has))&&!alt_has);
        release(alt_iterator);release(alt_iterable);release(alternates);
        del(wordtext);release(word);
    }
    void *word=NULL;UINT32 span[2]={0};
    CHECK(SUCCEEDED(CALL(seg,7,HRESULT(WINAPI *)(void *,HS,UINT32,void **))(seg,text,9,&word))&&word);
    CHECK(SUCCEEDED(CALL(word,7,HRESULT(WINAPI *)(void *,UINT32 *))(word,span))&&span[0]==11);release(word);
    CHECK(CALL(seg,7,HRESULT(WINAPI *)(void *,HS,UINT32,void **))(seg,text,lstrlenW(input),&word)==(HRESULT)0x8000000b);
    void *iterable=NULL,*iterator=NULL;
    CHECK(SUCCEEDED(CALL(view,0,HRESULT(WINAPI *)(void *,REFIID,void **))(view,&iterable_iid,&iterable)));
    CHECK(SUCCEEDED(CALL(iterable,6,HRESULT(WINAPI *)(void *,void **))(iterable,&iterator)));
    release(iterable);release(view);release(seg);del(text);
    BOOLEAN has=FALSE;CHECK(SUCCEEDED(CALL(iterator,7,HRESULT(WINAPI *)(void *,BOOLEAN *))(iterator,&has))&&has);
    CHECK(SUCCEEDED(CALL(iterator,6,HRESULT(WINAPI *)(void *,void **))(iterator,&word)));release(word);
    release(iterator);puts("PASS: WordsSegmenter Unicode offsets, GetTokenAt, collections and lifetime");return 0;
}
