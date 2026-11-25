/*
 * ms365ink.c — Minimal 64-bit Tablet PC Ink COM stub (CLSID_InkDisp et al.)
 * Satisfies Microsoft OneNote's "Desktop Experience" startup check under Wine/GE-Proton.
 */
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <unknwn.h>
#include <oaidl.h>

/* CLSID_InkDisp      {937C1A34-151D-4610-9CA6-A8CC9BDB5D83} */
static const GUID CLSID_InkDisp_ = {
    0x937C1A34, 0x151D, 0x4610, {0x9C, 0xA6, 0xA8, 0xCC, 0x9B, 0xDB, 0x5D, 0x83}
};
/* CLSID_InkCollector {43FB1553-AD74-4EE8-88E4-3E6DAAC915DB} */
static const GUID CLSID_InkCollector_ = {
    0x43FB1553, 0xAD74, 0x4EE8, {0x88, 0xE4, 0x3E, 0x6D, 0xAA, 0xC9, 0x15, 0xDB}
};
/* CLSID_InkOverlay   {65D00646-CDE3-4A88-9163-6769F0F1A97D} */
static const GUID CLSID_InkOverlay_ = {
    0x65D00646, 0xCDE3, 0x4A88, {0x91, 0x63, 0x67, 0x69, 0xF0, 0xF1, 0xA9, 0x7D}
};
/* CLSID_InkRenderer  {9C1CC6E4-D7EB-4EEB-9091-15A7C3791601} */
static const GUID CLSID_InkRenderer_ = {
    0x9C1CC6E4, 0xD7EB, 0x4EEB, {0x90, 0x91, 0x15, 0xA7, 0xC3, 0x79, 0x16, 0x01}
};

/* Generic COM stub object with 64 vtable slots (IUnknown + IDispatch + IInk* methods) */
typedef struct {
    void **lpVtbl;
    LONG ref;
} InkStubObj;

static HRESULT STDMETHODCALLTYPE InkStub_QueryInterface(InkStubObj *This, REFIID riid, void **ppv) {
    if (!ppv) return E_POINTER;
    /* Reject IMarshal / IStdMarshalInfo / IAgileObject? No, accept IUnknown, IDispatch, and Ink IIDs */
    if (IsEqualIID(riid, &IID_IMarshal)) {
        *ppv = NULL;
        return E_NOINTERFACE;
    }
    InterlockedIncrement(&This->ref);
    *ppv = This;
    return S_OK;
}

static ULONG STDMETHODCALLTYPE InkStub_AddRef(InkStubObj *This) {
    return (ULONG)InterlockedIncrement(&This->ref);
}

static ULONG STDMETHODCALLTYPE InkStub_Release(InkStubObj *This) {
    ULONG r = (ULONG)InterlockedDecrement(&This->ref);
    if (r == 0) {
        HeapFree(GetProcessHeap(), 0, This);
    }
    return r;
}

static HRESULT STDMETHODCALLTYPE InkStub_NotImpl(void *This) {
    (void)This;
    return E_NOTIMPL;
}

static void *g_ink_vtbl[64];

static void init_ink_vtbl(void) {
    if (g_ink_vtbl[0]) return;
    g_ink_vtbl[0] = (void *)InkStub_QueryInterface;
    g_ink_vtbl[1] = (void *)InkStub_AddRef;
    g_ink_vtbl[2] = (void *)InkStub_Release;
    for (int i = 3; i < 64; i++) {
        g_ink_vtbl[i] = (void *)InkStub_NotImpl;
    }
}

/* IClassFactory */
typedef struct {
    IClassFactoryVtbl *lpVtbl;
} InkClassFactory;

static HRESULT STDMETHODCALLTYPE CF_QueryInterface(IClassFactory *This, REFIID riid, void **ppv) {
    if (!ppv) return E_POINTER;
    if (IsEqualIID(riid, &IID_IUnknown) || IsEqualIID(riid, &IID_IClassFactory)) {
        *ppv = This;
        return S_OK;
    }
    *ppv = NULL;
    return E_NOINTERFACE;
}

static ULONG STDMETHODCALLTYPE CF_AddRef(IClassFactory *This) { (void)This; return 2; }
static ULONG STDMETHODCALLTYPE CF_Release(IClassFactory *This) { (void)This; return 1; }

static HRESULT STDMETHODCALLTYPE CF_CreateInstance(
    IClassFactory *This, IUnknown *pUnkOuter, REFIID riid, void **ppv
) {
    (void)This;
    if (!ppv) return E_POINTER;
    *ppv = NULL;
    if (pUnkOuter) return CLASS_E_NOAGGREGATION;

    init_ink_vtbl();
    InkStubObj *obj = (InkStubObj *)HeapAlloc(GetProcessHeap(), HEAP_ZERO_MEMORY, sizeof(InkStubObj));
    if (!obj) return E_OUTOFMEMORY;
    obj->lpVtbl = g_ink_vtbl;
    obj->ref = 1;

    HRESULT hr = InkStub_QueryInterface(obj, riid, ppv);
    InkStub_Release(obj);
    return hr;
}

static HRESULT STDMETHODCALLTYPE CF_LockServer(IClassFactory *This, BOOL fLock) {
    (void)This;
    (void)fLock;
    return S_OK;
}

static IClassFactoryVtbl g_cf_vtbl = {
    CF_QueryInterface,
    CF_AddRef,
    CF_Release,
    CF_CreateInstance,
    CF_LockServer,
};

static InkClassFactory g_cf = { &g_cf_vtbl };

__declspec(dllexport) HRESULT WINAPI DllGetClassObject(REFCLSID rclsid, REFIID riid, LPVOID *ppv) {
    if (!ppv) return E_POINTER;
    *ppv = NULL;
    if (IsEqualCLSID(rclsid, &CLSID_InkDisp_) ||
        IsEqualCLSID(rclsid, &CLSID_InkCollector_) ||
        IsEqualCLSID(rclsid, &CLSID_InkOverlay_) ||
        IsEqualCLSID(rclsid, &CLSID_InkRenderer_)) {
        return CF_QueryInterface((IClassFactory *)&g_cf, riid, ppv);
    }
    return CLASS_E_CLASSNOTAVAILABLE;
}

__declspec(dllexport) HRESULT WINAPI DllCanUnloadNow(void) {
    return S_FALSE;
}

BOOL WINAPI DllMain(HINSTANCE hinst, DWORD reason, LPVOID reserved) {
    (void)hinst;
    (void)reserved;
    if (reason == DLL_PROCESS_ATTACH) {
        DisableThreadLibraryCalls(hinst);
        init_ink_vtbl();
    }
    return TRUE;
}
