/* Windows.Data.Text.WordsSegmenter compatibility for OneNote's text loader.
 * ABI: microsoft/win32metadata Windows.Data.Text.h. Word boundaries use the
 * runner's ICU, preserving UTF-16 source offsets, combining marks and CJK rules.
 * Alternate linguistic forms are unavailable; their collection is empty.
 * Included by shim.c after resolve_winrt/hstring_of.
 */
static const GUID T_FACTORY = {0xe6977274,0xfc35,0x455c,{0x8b,0xfb,0x6d,0x7f,0x46,0x53,0xca,0x97}};
static const GUID T_SEGMENTER = {0x86b4d4d1,0xb2fe,0x4e34,{0xa8,0x1d,0x66,0x64,0x03,0x00,0x45,0x4f}};
static const GUID T_WORD = {0xd2d4ba6d,0x987c,0x4cc0,{0xb6,0xbd,0xd4,0x9a,0x11,0xb3,0x8f,0x9a}};
static const GUID T_VIEW = {0xc706749a,0xe11d,0x5e07,{0x85,0x34,0x2b,0xd2,0x3e,0xc2,0x10,0xf9}};
static const GUID T_ITERABLE = {0xd14dc94a,0xa311,0x5ddc,{0xbf,0x8b,0xd5,0x87,0x95,0xd8,0x7d,0x6d}};
static const GUID T_ITERATOR = {0xaeb846b3,0x664b,0x545e,{0xab,0x93,0xf5,0xfc,0x66,0xd2,0x4e,0x32}};
static const GUID T_ALT_VIEW = {0x6b742ff2,0x746a,0x5545,{0xa6,0xed,0x3b,0xba,0x45,0x3c,0xf5,0xd9}};
static const GUID T_ALT_ITERABLE = {0xae838c29,0x60c1,0x5093,{0xae,0x61,0x0e,0xd2,0x28,0x57,0xdb,0x05}};
static const GUID T_ALT_ITERATOR = {0x7f463f8a,0xe08f,0x516e,{0xb4,0xb8,0x81,0xb5,0x41,0x7b,0xfb,0x58}};
static const GUID T_AGILE = {0x94ea2b94,0xe9cc,0x49e0,{0xc0,0xff,0xee,0x64,0xca,0x8f,0x5b,0x90}};
#define T_BOUNDS ((HRESULT)0x8000000b)
struct t_obj;
struct t_if { void *const *vt; struct t_obj *obj; };
struct t_obj {
    struct t_if ifs[2]; const GUID *iids[2]; int n;
    LONG ref; const WCHAR *cls; WCHAR *text;
    UINT32 start, length, count, position;
    struct t_obj **items, *owner;
};
static ULONG WINAPI t_add(struct t_if *s) { return InterlockedIncrement(&s->obj->ref); }
static ULONG WINAPI t_release(struct t_if *s)
{
    struct t_obj *o=s->obj; LONG r=InterlockedDecrement(&o->ref);
    if (!r) {
        for (UINT32 i=0;i<o->count;i++) t_release(&o->items[i]->ifs[0]);
        if (o->owner) t_release(&o->owner->ifs[0]);
        HeapFree(GetProcessHeap(),0,o->items); HeapFree(GetProcessHeap(),0,o->text);
        HeapFree(GetProcessHeap(),0,o);
    }
    return r;
}
static HRESULT WINAPI t_qi(struct t_if *s, REFIID iid, void **out)
{
    if (!out) return E_POINTER; *out=NULL;
    if (IsEqualGUID(iid,&IID_IUnknown)||IsEqualGUID(iid,&MY_IID_IInspectable)||IsEqualGUID(iid,&T_AGILE)) *out=&s->obj->ifs[0];
    for (int i=0;i<s->obj->n;i++) if (IsEqualGUID(iid,s->obj->iids[i])) *out=&s->obj->ifs[i];
    if (!*out) return E_NOINTERFACE; t_add(s); return S_OK;
}
static HRESULT WINAPI t_ids(struct t_if *s, ULONG *n, GUID **ids)
{
    if (!n||!ids) return E_POINTER; *n=0; *ids=NULL;
    void *(WINAPI *allocate)(SIZE_T)=(void *)real_GetProcAddress(GetModuleHandleA("combase.dll"),"CoTaskMemAlloc");
    if (!allocate) return E_NOTIMPL;
    *ids=allocate(sizeof(GUID)*s->obj->n);
    if (!*ids) return E_OUTOFMEMORY;
    for (int i=0;i<s->obj->n;i++) (*ids)[i]=*s->obj->iids[i]; *n=s->obj->n; return S_OK;
}
static HRESULT WINAPI t_class(struct t_if *s,HSTRING *out) { return hstring_of(s->obj->cls,out); }
static HRESULT WINAPI t_trust(struct t_if *s,int *out) { (void)s; if (!out) return E_POINTER; *out=0;return S_OK; }
#define T_BASE (void *)t_qi,(void *)t_add,(void *)t_release,(void *)t_ids,(void *)t_class,(void *)t_trust
static struct t_obj *t_new(const WCHAR *cls,void *const *vt,const GUID *iid)
{
    struct t_obj *o=HeapAlloc(GetProcessHeap(),HEAP_ZERO_MEMORY,sizeof(*o));
    if (o) { o->ref=1;o->cls=cls;o->n=1;o->ifs[0].vt=vt;o->ifs[0].obj=o;o->iids[0]=iid; } return o;
}
static WCHAR *t_copy(const WCHAR *text,UINT32 len)
{
    if ((SIZE_T)len>(SIZE_MAX/sizeof(WCHAR))-1) return NULL;
    WCHAR *p=HeapAlloc(GetProcessHeap(),0,((SIZE_T)len+1)*sizeof(WCHAR));
    if (p) { if(len) memcpy(p,text,(SIZE_T)len*sizeof(WCHAR));p[len]=0; } return p;
}
static HRESULT WINAPI t_text(struct t_if *s,HSTRING *out)
{ if(!out)return E_POINTER;return p_WindowsCreateString(s->obj->text,s->obj->length,out); }
struct t_span { UINT32 start,length; };
static HRESULT WINAPI t_source(struct t_if *s,struct t_span *out)
{ if(!out)return E_POINTER;out->start=s->obj->start;out->length=s->obj->length;return S_OK; }
static struct t_obj *t_vector(BOOL alternate);
static HRESULT WINAPI t_alternates(struct t_if *s,void **out)
{ (void)s;if(!out)return E_POINTER;struct t_obj *o=t_vector(TRUE);*out=o?&o->ifs[0]:NULL;return o?S_OK:E_OUTOFMEMORY; }
static void *const t_word_vt[]={T_BASE,(void *)t_text,(void *)t_source,(void *)t_alternates};
static HRESULT WINAPI t_get(struct t_if *s,UINT32 index,void **out)
{
    if(!out)return E_POINTER;*out=NULL;if(index>=s->obj->count)return T_BOUNDS;
    *out=&s->obj->items[index]->ifs[0];t_add(*out);return S_OK;
}
static HRESULT WINAPI t_size(struct t_if *s,UINT32 *out)
{if(!out)return E_POINTER;*out=s->obj->count;return S_OK;}
static HRESULT WINAPI t_index(struct t_if *s,void *item,UINT32 *index,BOOLEAN *found)
{
    if(!index||!found)return E_POINTER;*index=0;*found=FALSE;
    for(UINT32 i=0;i<s->obj->count;i++)if(item==&s->obj->items[i]->ifs[0]){*index=i;*found=TRUE;break;}return S_OK;
}
static HRESULT WINAPI t_many(struct t_if *s,UINT32 start,UINT32 capacity,void **items,UINT32 *actual)
{
    if(!actual||(!items&&capacity))return E_POINTER;*actual=0;if(start>s->obj->count)return T_BOUNDS;
    while(*actual<capacity&&start+*actual<s->obj->count){t_get(s,start+*actual,&items[*actual]);(*actual)++;}return S_OK;
}
static HRESULT WINAPI t_current(struct t_if *s,void **out){return t_get(&s->obj->owner->ifs[0],s->obj->position,out);}
static HRESULT WINAPI t_has(struct t_if *s,BOOLEAN *out)
{if(!out)return E_POINTER;*out=s->obj->position<s->obj->owner->count;return S_OK;}
static HRESULT WINAPI t_next(struct t_if *s,BOOLEAN *out)
{if(s->obj->position<s->obj->owner->count)s->obj->position++;return t_has(s,out);}
static HRESULT WINAPI t_iter_many(struct t_if *s,UINT32 capacity,void **items,UINT32 *actual)
{HRESULT hr=t_many(&s->obj->owner->ifs[0],s->obj->position,capacity,items,actual);if(SUCCEEDED(hr))s->obj->position+=*actual;return hr;}
static void *const t_iterator_vt[]={T_BASE,(void *)t_current,(void *)t_has,(void *)t_next,(void *)t_iter_many};
static HRESULT WINAPI t_first(struct t_if *s,void **out)
{
    if(!out)return E_POINTER;*out=NULL;
    BOOL alternate=IsEqualGUID(s->obj->iids[0],&T_ALT_VIEW);
    struct t_obj *o=t_new(alternate?L"Windows.Foundation.Collections.IIterator`1<Windows.Data.Text.AlternateWordForm>":L"Windows.Foundation.Collections.IIterator`1<Windows.Data.Text.WordSegment>",t_iterator_vt,alternate?&T_ALT_ITERATOR:&T_ITERATOR);
    if(!o)return E_OUTOFMEMORY;o->owner=s->obj;t_add(s);*out=&o->ifs[0];return S_OK;
}
static void *const t_view_vt[]={T_BASE,(void *)t_get,(void *)t_size,(void *)t_index,(void *)t_many};
static void *const t_iterable_vt[]={T_BASE,(void *)t_first};
static struct t_obj *t_vector(BOOL alternate)
{
    struct t_obj *o=t_new(alternate?L"Windows.Foundation.Collections.IVectorView`1<Windows.Data.Text.AlternateWordForm>":L"Windows.Foundation.Collections.IVectorView`1<Windows.Data.Text.WordSegment>",t_view_vt,alternate?&T_ALT_VIEW:&T_VIEW);
    if(o){o->n=2;o->ifs[1].vt=t_iterable_vt;o->ifs[1].obj=o;o->iids[1]=alternate?&T_ALT_ITERABLE:&T_ITERABLE;}return o;
}
static BOOL t_append(struct t_obj *v,struct t_obj *word)
{
    SIZE_T size=((SIZE_T)v->count+1)*sizeof(*v->items);
    void *items=v->items?HeapReAlloc(GetProcessHeap(),0,v->items,size):HeapAlloc(GetProcessHeap(),0,size);
    if(!items)return FALSE;v->items=items;v->items[v->count++]=word;return TRUE;
}
/* ICU's Windows C ABI is __cdecl. Keep the module loaded while these pointers live. */
static void *(__cdecl *t_break_open)(int,const char *,const WCHAR *,int,int *);
static int (__cdecl *t_break_first)(void *),(__cdecl *t_break_next)(void *),(__cdecl *t_break_status)(void *);
static void (__cdecl *t_break_close)(void *);
static INIT_ONCE t_icu_once=INIT_ONCE_STATIC_INIT;
static BOOL CALLBACK t_icu_init(PINIT_ONCE once,void *param,void **context)
{
    (void)once;(void)param;(void)context;HMODULE m=LoadLibraryA("icu.dll");
    if(m){
        t_break_open=(void *)real_GetProcAddress(m,"ubrk_open");t_break_first=(void *)real_GetProcAddress(m,"ubrk_first");
        t_break_next=(void *)real_GetProcAddress(m,"ubrk_next");t_break_status=(void *)real_GetProcAddress(m,"ubrk_getRuleStatus");
        t_break_close=(void *)real_GetProcAddress(m,"ubrk_close");
    }return TRUE;
}
static HRESULT WINAPI t_tokens(struct t_if *s,HSTRING text,void **out)
{
    if(!out)return E_POINTER;*out=NULL;UINT32 len=0;
    const WCHAR *str=p_WindowsGetStringRawBuffer(text,&len);if(len>INT_MAX)return E_INVALIDARG;
    InitOnceExecuteOnce(&t_icu_once,t_icu_init,NULL,NULL);
    if(!t_break_open||!t_break_first||!t_break_next||!t_break_status||!t_break_close)return E_NOTIMPL;
    char locale[256];if(!WideCharToMultiByte(CP_UTF8,0,s->obj->text,-1,locale,sizeof(locale),NULL,NULL))return E_INVALIDARG;
    int status=0;void *br=t_break_open(1 /* UBRK_WORD */,locale,str,len,&status);
    if(status>0||!br)return E_INVALIDARG;
    struct t_obj *v=t_vector(FALSE);HRESULT hr=v?S_OK:E_OUTOFMEMORY;
    int start=t_break_first(br),end;
    while(v&&(end=t_break_next(br))!=-1){
        if(t_break_status(br)>=100){
            struct t_obj *word=t_new(L"Windows.Data.Text.WordSegment",t_word_vt,&T_WORD);
            if(!word){hr=E_OUTOFMEMORY;break;}
            word->start=start;word->length=end-start;word->text=t_copy(str+start,end-start);
            if(!word->text||!t_append(v,word)){t_release(&word->ifs[0]);hr=E_OUTOFMEMORY;break;}
        }start=end;
    }
    t_break_close(br);if(FAILED(hr)){if(v)t_release(&v->ifs[0]);return hr;}*out=&v->ifs[0];return S_OK;
}
static HRESULT WINAPI t_token_at(struct t_if *s,HSTRING text,UINT32 index,void **out)
{
    if(!out)return E_POINTER;*out=NULL;UINT32 len;p_WindowsGetStringRawBuffer(text,&len);if(index>=len)return T_BOUNDS;
    struct t_if *v=NULL;HRESULT hr=t_tokens(s,text,(void **)&v);if(FAILED(hr))return hr;
    for(UINT32 i=0;i<v->obj->count;i++){struct t_obj *w=v->obj->items[i];if(w->start+w->length>index){t_get(v,i,out);break;}}
    t_release(v);return S_OK;
}
static HRESULT WINAPI t_tokenize(struct t_if *s,HSTRING text,UINT32 index,void *handler)
{
    if(!handler)return E_POINTER;UINT32 len;p_WindowsGetStringRawBuffer(text,&len);if(index>len)return T_BOUNDS;
    struct t_if *all=NULL;HRESULT hr=t_tokens(s,text,(void **)&all);if(FAILED(hr))return hr;
    struct t_obj *before=t_vector(FALSE),*after=t_vector(FALSE);
    if(!before||!after)hr=E_OUTOFMEMORY;
    for(UINT32 i=0;SUCCEEDED(hr)&&i<all->obj->count;i++){
        struct t_obj *w=all->obj->items[i];struct t_obj *v=w->start+w->length<=index?before:after;
        if(!t_append(v,w)){hr=E_OUTOFMEMORY;break;}t_add(&w->ifs[0]);
    }
    if(SUCCEEDED(hr))hr=((HRESULT (WINAPI *)(void *,void *,void *))(*(void ***)handler)[3])(handler,&before->ifs[1],&after->ifs[1]);
    if(before)t_release(&before->ifs[0]);if(after)t_release(&after->ifs[0]);t_release(all);return hr;
}
static void *const t_segmenter_vt[]={T_BASE,(void *)t_text,(void *)t_token_at,(void *)t_tokens,(void *)t_tokenize};
static HRESULT WINAPI t_create(struct t_if *s,HSTRING language,void **out)
{
    (void)s;if(!out)return E_POINTER;*out=NULL;UINT32 len=0;const WCHAR *lang=p_WindowsGetStringRawBuffer(language,&len);
    if(!len||len>=256)return E_INVALIDARG;
    struct t_obj *o=t_new(L"Windows.Data.Text.WordsSegmenter",t_segmenter_vt,&T_SEGMENTER);if(!o)return E_OUTOFMEMORY;
    o->length=len;o->text=t_copy(lang,len);if(!o->text){t_release(&o->ifs[0]);return E_OUTOFMEMORY;}
    *out=&o->ifs[0];return S_OK;
}
static void *const t_factory_vt[]={T_BASE,(void *)t_create};
static HRESULT t_factory(REFIID iid,void **out)
{
    if(!out)return E_POINTER;*out=NULL;if(!IsEqualGUID(iid,&T_FACTORY))return E_NOINTERFACE;
    struct t_obj *o=t_new(L"Windows.Data.Text.WordsSegmenter",t_factory_vt,&T_FACTORY);if(!o)return E_OUTOFMEMORY;
    *out=&o->ifs[0];return S_OK;
}
