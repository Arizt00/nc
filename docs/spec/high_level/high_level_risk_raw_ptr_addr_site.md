# High Level Risk Raw Ptr Addr Site

Raw pointer, raw address, and raw site authority are separate because their failure modes differ. Under Pointer Model V2, `ptr<T>`, raw address projection, and raw site intent are C/C++-grade operational paths by default on pointer-capable nC surfaces. The `risk` section remains useful as audit metadata and policy documentation, but it is no longer the gate that permits raw pointer operation.

```n
risk:
    allow rawPtr;
    accept rawPtr "foreignSlice";
    allow rawAddr;
    accept rawAddr "memoryMappedRegister";
    allow rawSite;
    accept rawSite "deviceSite";
```

Diagnostics:

- `E_HIGH_RISK_RAW_PTR_001`
- `E_HIGH_RISK_RAW_ADDR_001`
- `E_HIGH_RISK_RAW_SITE_001`

These diagnostics are retained for legacy strict-policy reports and non-pointer-capable surfaces. Pointer Model V2 keeps `E_PTR_*` diagnostics for null dereference, stale use, bounded-view overflow, bad alignment, or invalid provenance. A raw pointer leaving its allocation is permitted and lowers directly to backend address arithmetic.
