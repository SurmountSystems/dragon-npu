# Ryzen AI eepy-min (ORT Vitis EP without 11 GiB)

## Purpose

Run AMD’s **ONNX Runtime `VitisAIExecutionProvider`** on XDNA2 (STX/KRK,
`1022:17f0`) **without** every teammate downloading the official
`ryzen_ai-1.7.1` dump (~**11 GiB** of telluride / Vitis AIE toolchain / llvm /
full flexml / torch / …).

This is the **vendor EP path**. It is **not** dragon-npu pure-direct
`EXEC_CMD` / txn research (`docs/XDNA2_TXN_RESEARCH.md`).

## Where the tooling lives

Outside this git tree (binaries + AMD EULA wheels must not be committed):

```text
~/Projects/ai/ryzen-ai-install/
  pack_eepy_min.sh       # once: strip full dump → minimal-wheels/
  share_eepy_min.sh      # once: eepy-min-share-*.tar.xz for friends
  install_eepy_min.sh    # every machine: ~460 MiB wheels → ~1.4 GiB venv
  activate-eepy.sh       # source me
  smoke_eepy_min.sh      # providers + CPU session (no NPU EXEC required)
  minimal-wheels/        # ~460 MiB
  eepy-min-share-*.tar.xz
  venv-eepy-min/         # after install
  eepy.so → libonnxruntime_vitisai_ep.so
```

Full dump (packer only): `~/Downloads/ryzen_ai-1.7.1` (~11 GiB).

## Size table

| Artifact | ~size | Role |
|----------|-------|------|
| Official dump | 11 GiB | **One** download to pack |
| Share tarball | ~440 MiB | What teammates get |
| Installed min venv | ~1.4 GiB | Runtime |
| Full official venv | 20 GiB+ | Avoid |

## Teammate install (no 11 GiB)

```bash
# receive eepy-min-share-YYYYMMDD.tar.xz (private share, AMD EULA)
tar -xJf eepy-min-share-YYYYMMDD.tar.xz
cd eepy-min
# prerequisites: uv, Python 3.12; Arch: pacman -S xrt xrt-plugin-amdxdna
bash install_eepy_min.sh
source ./activate-eepy.sh
# expect: ['VitisAIExecutionProvider', 'CPUExecutionProvider']
bash smoke_eepy_min.sh
```

## Packer (once, needs full dump)

```bash
cd ~/Projects/ai/ryzen-ai-install
bash pack_eepy_min.sh ~/Downloads/ryzen_ai-1.7.1
bash share_eepy_min.sh
```

Excluded bulk is listed in `minimal-wheels/EXCLUDED.md`.

## Hardened driver / live NPU

dragon-npu L3 defaults `exec_cmd_allow=0` → EP **inference** may hit
`-EPERM` on `EXEC_CMD`. Provider list and CPU sessions still work. Lab-only
temporary allow=1 (then restore 0): `docs/XDNA2_NEVER_AGAIN.md`,
`docs/XDNA2_EXEC_CMD_LAB.md`.

## License

AMD EULA covers the wheels. **Private / org redistribution only** unless your
license says otherwise. **Do not** public-CDN rehost AMD binaries.

## Relation to pure-direct

| Path | What it is | Needs eepy-min? |
|------|------------|-----------------|
| Pure direct (this repo) | amdxdna ioctls, txn research, L1/L2/L3 gates | No |
| ORT Vitis EP (eepy) | Vendor plugin for ONNX Runtime | Yes (min pack) |

Use eepy-min when you want a known-good vendor EP for app bring-up (e.g.
Sovcit / fc-pregmod style ORT). Use pure-direct when building the lightweight
stack that never depends on that bulk.
