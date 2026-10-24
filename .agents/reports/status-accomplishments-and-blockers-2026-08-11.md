# Status: accomplishments and blockers (2026-08-11)

**invent_pass=0 · Pass=No · no active unlock**

## Done

- Offline residual honesty: dual/triple public npu2 AOT under
  `testdata/xdna2_cmd/external_known_good/` tagged **RESEARCH ONLY**
  (`public-npu2-small` vs `flexml-2x4x4-class` vs `mladf-classic-1688`).
- Live L: flexml llm-2x4x4 under `8352882b` CONSUMED; attach OK;
  `wait_cmd_einval` + IO_PAGE_FAULT ~`0x4fff8000…`.
- Live U: flexml unified-4x4 under `efd44f35` CONSUMED; attach OK;
  `wait_cmd_einval` + IO_PAGE_FAULT ~`0x8fff8000…` (TXN OP `0xd`).
- All three flexml STX research PDIs **Spent**. Join:
  `.agents/reports/impl-feedback-dual-live-join-closeout.md`.

## Blocked for Pass

- Still no ERT Completed + non-zero OFM allclose on real ioctl.
- No active `C_UNLOCK`. Spent fingerprints must not be re-lived alone.
- Public research peels are not residual-matched load identity.

## Next (not auto-queued)

Optional offline IO_PAGE_FAULT band vs BO map; new material before any new
unlock. No empty reconfirm mill.
