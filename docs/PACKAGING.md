# crates.io packaging, versioning, and MSRV

Policy for publishing and CI of the **Rust crate** `dragon-npu`. Kernel patches
and lab scripts live in the same git repo but are not the crates.io product.

**Do not** `cargo publish` from automation without the package owner’s explicit
intent. This document is policy only.

## Version policy

| Range | Meaning |
|-------|---------|
| `0.1.x` | Pre silicon-MVP: simulation solid; XDNA2 direct bring-up; no claimed `ran_on_npu` GEMM Pass |
| `0.2.x` (future) | After known-good txn + numerical verify path exists end-to-end |
| `1.x` (future) | Stable public API commitment |

- Follow [semver](https://semver.org/) for `0.y.z`: breaking changes may bump `y`.
- Feature flags are part of the public surface; enabling a feature must not
  silently claim hardware success (see honesty rules).

## Feature flags (publish surface)

| Feature | Default | Role |
|---------|---------|------|
| `simulation` | **yes** (in `default`) | Host simulation backend |
| `amd-xdna2` | no | Primary hardware: lightweight amdxdna direct |
| `xdna2-dangerous-exec-cmd` | no | L1 compile gate for EXEC_CMD (also needs env) |
| `intel-vpu` | no | Honest stub |
| `qualcomm-hexagon` | no | Honest stub |
| `xrt-bridge` | no | Research stub only — **never** production default |

`xrt-bridge` is **not** enabled by `amd-xdna2`. Secondary stubs are **not**
selected by `DragonNpu::new`.

## MSRV

- `Cargo.toml` sets `rust-version = "1.99"`.
  - Edition 2024 alone needs ≥1.85.
  - This crate uses APIs stabilized in 1.87 (`usize::is_multiple_of`,
    integer `cast_unsigned`). The documented floor is **1.99**, the
    stable this crate is built and tested on.
- CI / lab may use a newer stable; MSRV is the documented floor.
- Bump MSRV only with a deliberate changelog note.

## What is published vs git-only

| In crates.io tarball (typical) | Git / lab only (excluded or out of band) |
|--------------------------------|------------------------------------------|
| `src/`, `tests/fixtures/`, `benches/`, `examples/` | `kernel/out/*.ko`, `kernel/dkms/staging/` |
| `docs/*.md` design docs as needed | `docs/upstream/evidence/` host snapshots |
| `Cargo.toml`, `LICENSE`, `README.md` | Full `dragon_npu/` Python tree (reference; excluded) |
| `kernel/patches/*.patch` + small README (optional size) | Large rescue `.ko` / object files |

`exclude` in `Cargo.toml` keeps large/host-specific trees out of the package.
Kernel **patches** may still ship as documentation for OOT builds; they are not
a runtime dependency of the crate.

## Gate commands (no hardware required)

```bash
export PATH=/usr/bin:$PATH

cargo fmt -- --check
cargo clippy -- -D warnings
cargo clippy --features amd-xdna2 -- -D warnings
cargo clippy --features intel-vpu,qualcomm-hexagon -- -D warnings
cargo clippy --features xrt-bridge -- -D warnings

cargo test --lib
cargo test --features amd-xdna2 --lib
cargo test --features intel-vpu,qualcomm-hexagon --lib
cargo test --features xrt-bridge --lib

cargo build --bin dragon-npu
cargo bench --no-run
```

No test in this matrix should require `XDNA2_ALLOW_EXEC_CMD=1`, modprobe, or a
Pass claim for silicon GEMM.

## CI

See [`.github/workflows/ci.yml`](../.github/workflows/ci.yml) (software-only):

| Job | Role |
|-----|------|
| `fmt` | `cargo fmt -- --check` on stable |
| `msrv` | `rust-toolchain@1.99` — `cargo test --lib` and `cargo test --lib --features amd-xdna2` |
| `clippy-test` | matrix of features; `clippy --all-targets -D warnings` + `test --lib` |
| `exec-gate` | clippy and `test --lib` with `amd-xdna2,xdna2-dangerous-exec-cmd`; `XDNA2_ALLOW_EXEC_CMD` stays unset |
| `offline-gates` | `bash tools/run_offline_gates.sh` (soft assets; not C-ready; no execute allow) |
| `lab-script` | `bash -n` on lab and extract scripts, plus dry-run refuses; never `--apply` |
| `bin` | `build --bin dragon-npu`; clippy bins with `amd-xdna2`; `bench --no-run` |

PATH: steps use `PATH="/usr/bin:$PATH"` (prepend only) so cargo/rustup from the
runner are not wiped by a workflow-level absolute PATH.

## Related

- Secondary backends: [`SECONDARY_BACKENDS.md`](./SECONDARY_BACKENDS.md)
- DKMS vs upstream: [`XDNA2_PACKAGING.md`](./XDNA2_PACKAGING.md)
- Safety: [`XDNA2_NEVER_AGAIN.md`](./XDNA2_NEVER_AGAIN.md)
