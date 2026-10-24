# PDI / xclbin inventory (read-only; S5)

**Date:** 2026-08-10T14:44:57-06:00

## Honesty

- invent_pass=0
- not product known-good; empty result is OK
- paths and sizes only; no proprietary blobs copied into git
- missing install roots are not errors
- **xclbin-only by design** (plan A4: `*mladf*.xclbin` / `*gemm*.xclbin`; txn bins are ctrltext, not PDI)
- maxdepth=12

## Search roots

- present: `/home/hunter/Projects/ai/ryzen-ai-install`
- present: `/home/hunter/Projects/ai/RyzenAI-SW`

## Matches (`*mladf*.xclbin` / `*gemm*.xclbin` only)

| path | bytes |
|------|-------|

**Count:** 0 — no matching xclbins under present roots (honest empty).

## Footer

invent_pass=0 product_known_good=0 not_ran_on_npu=1
Note: S5 is **xclbin-only by design**. DynamicDispatch txn bins are ctrltext, not PDI.
