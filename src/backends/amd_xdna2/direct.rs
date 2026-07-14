//! Lightweight, direct driver implementation for AMD XDNA2.
//!
//! This module contains the scaffolding for a clean, open, lower-level
//! implementation that talks directly to the amdxdna kernel driver via
//! the file descriptor we already manage in `RealXdna2Device`.
//!
//! Design goals (per project direction):
//! - Lighter weight than official heavy runtimes (XRT, etc.)
//! - Focused and maintainable
//! - Eventually more performant by avoiding unnecessary layers
//! - Uses only the mainline amdxdna uAPI (ioctls exposed in amdxdna_accel.h)
//!
//! Current status:
//! - Typed high-level diagnostic queries (firmware, power, resources, sensors, etc.)
//!   via DRM_IOCTL_AMDXDNA_GET_INFO are fully wired and return real data on hardware.
//! - The DirectSubmission trait has ported structs for the full submission path
//!   (create_hwctx, BO management, EXEC_CMD, etc.).
//! - Real ioctl implementation for BOs + hwctx creation is the highest-leverage
//!   next layer. Command payload format discovery remains the hardest unknown
//!   for a pure lightweight direct implementation.
//!
//! Key ioctls we will eventually use (from amdxdna_accel.h):
//! - DRM_IOCTL_AMDXDNA_CREATE_HWCTX
//! - DRM_IOCTL_AMDXDNA_DESTROY_HWCTX
//! - DRM_IOCTL_AMDXDNA_CREATE_BO
//! - DRM_IOCTL_AMDXDNA_EXEC_CMD
//!   etc.

use std::fmt::Write as _;
use std::os::unix::io::RawFd;

/// DRM_AMDXDNA_CREATE_HWCTX
pub const DRM_AMDXDNA_CREATE_HWCTX: u32 = 0;
/// DRM_AMDXDNA_DESTROY_HWCTX
pub const DRM_AMDXDNA_DESTROY_HWCTX: u32 = 1;
/// DRM_AMDXDNA_CREATE_BO
pub const DRM_AMDXDNA_CREATE_BO: u32 = 3;
/// DRM_AMDXDNA_EXEC_CMD
pub const DRM_AMDXDNA_EXEC_CMD: u32 = 6;
/// DRM_AMDXDNA_GET_INFO (the primary read-only query ioctl for firmware, power, resources, sensors, etc.)
pub const DRM_AMDXDNA_GET_INFO: u32 = 7;
/// DRM_AMDXDNA_SET_STATE
pub const DRM_AMDXDNA_SET_STATE: u32 = 8;
/// DRM_AMDXDNA_GET_ARRAY
pub const DRM_AMDXDNA_GET_ARRAY: u32 = 10;

#[allow(clippy::empty_line_after_doc_comments)]
/// Key uAPI structures (port of amdxdna_accel.h) for the direct lightweight path.
/// These are `#[repr(C)]` so they can be used with ioctl when we implement the real layer.

#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct amdxdna_drm_create_hwctx {
    pub ext: u64,
    pub ext_flags: u64,
    pub qos_p: u64,
    pub umq_bo: u32,
    pub log_buf_bo: u32,
    pub max_opc: u32,
    pub num_tiles: u32,
    pub mem_size: u32,
    pub umq_doorbell: u32,
    pub handle: u32,
    pub syncobj_handle: u32,
}

/// Direct port of struct amdxdna_drm_destroy_hwctx.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct amdxdna_drm_destroy_hwctx {
    pub handle: u32,
    pub pad: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct amdxdna_drm_exec_cmd {
    pub ext: u64,
    pub ext_flags: u64,
    pub hwctx: u32,
    pub ty: u32,
    pub cmd_handles: u64,
    pub args: u64,
    pub cmd_count: u32,
    pub arg_count: u32,
    pub seq: u64,
}

/// Buffer object types (direct port of enum amdxdna_bo_type from the uAPI).
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmdXdnaBoType {
    /// Invalid / uninitialized.
    Invalid = 0,
    /// Regular shared memory buffer visible to both host and device.
    Shmem = 1,
    /// Device heap buffer (used for large device-only allocations).
    DevHeap = 2,
    /// Device-only buffer.
    Dev = 3,
    /// Buffer used to hold commands for submission.
    Cmd = 4,
}

/// Direct port of struct amdxdna_drm_create_bo.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct amdxdna_drm_create_bo {
    pub flags: u64,
    pub vaddr: u64,
    pub size: u64,
    pub ty: u32,
    pub handle: u32,
}

/// Direct port of struct drm_gem_close (from drm.h).
///
/// amdxdna has **no** DESTROY_BO ioctl. Buffer objects are standard DRM GEM
/// objects and must be released with `DRM_IOCTL_GEM_CLOSE`.
/// (Earlier bring-up code incorrectly guessed nr=2 was DESTROY_BO; that nr is
/// actually `DRM_AMDXDNA_CONFIG_HWCTX`.)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct drm_gem_close {
    pub handle: u32,
    pub pad: u32,
}

/// Legacy alias kept so older call sites / docs that still say "destroy_bo"
/// compile. Prefer [`drm_gem_close`].
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct amdxdna_drm_destroy_bo {
    pub handle: u32,
    pub pad: u32,
}

/// Direct port of struct amdxdna_drm_get_bo_info.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct amdxdna_drm_get_bo_info {
    pub ext: u64,
    pub ext_flags: u64,
    pub handle: u32,
    pub pad: u32,
    pub map_offset: u64,
    pub vaddr: u64,
    pub xdna_addr: u64,
}

/// Direct port of struct amdxdna_drm_sync_bo.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct amdxdna_drm_sync_bo {
    pub handle: u32,
    pub direction: u32,
    pub offset: u64,
    pub size: u64,
}

// === Useful query / info structs (for get_info and diagnostics) ===

// -----------------------------------------------------------------------------
// Submission Path Reality Check (Lightweight Direct Only)
//
// The amdxdna uAPI exposes the necessary ioctls (CREATE_HWCTX, CREATE_BO,
// EXEC_CMD, etc.), but the *semantics* of what goes inside command BOs are
// not documented in the public headers.
//
// Real hardware contexts require:
// - Creating special BOs first (especially UMQ BO for the user mode queue +
//   a log buffer).
// - Calling CREATE_HWCTX (which returns a doorbell offset + syncobj).
// - Building command buffers (AMDXDNA_BO_CMD type) with a format the firmware
//   understands ("txn ops" / control code).
// - Submitting via EXEC_CMD with the right hwctx + command handles.
//
// The diagnostic side (GET_INFO queries) is now very solid.
// The submission side still requires discovery of the command payload format.
//
// This is the main remaining challenge for a pure lightweight direct implementation.
// -----------------------------------------------------------------------------

/// Direct port of struct amdxdna_drm_query_firmware_version.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct amdxdna_drm_query_firmware_version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub build: u32,
}

/// Direct port of struct amdxdna_drm_query_aie_version.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct amdxdna_drm_query_aie_version {
    pub major: u32,
    pub minor: u32,
}

/// Direct port of struct amdxdna_drm_get_power_mode.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct amdxdna_drm_get_power_mode {
    pub power_mode: u8,
    pub pad: [u8; 7],
}

/// Direct port of struct amdxdna_drm_query_sensor (single sensor).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct amdxdna_drm_query_sensor {
    pub label: [u8; 64],
    pub input: u32,
    pub max: u32,
    pub average: u32,
    pub highest: u32,
    pub status: [u8; 64],
    pub units: [u8; 16],
    pub unitm: i8,
    pub ty: u8,
    pub pad: [u8; 6],
}

/// Direct port of struct amdxdna_drm_get_resource_info (very useful for TOPS etc.).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct amdxdna_drm_get_resource_info {
    pub npu_clk_max: u64,
    pub npu_tops_max: u64,
    pub npu_task_max: u64,
    pub npu_tops_curr: u64,
    pub npu_task_curr: u64,
}

// The real lightweight direct implementation is being built incrementally here.
// - Diagnostic GET_INFO queries are already live and returning real data.
// - create_hwctx / create_buffer / submit_command are next (real ioctl calls
//   with the ported structs, kept honest about what is fully working).
// All work stays in the pure direct amdxdna style.

// Real ioctl implementation lives in the narrow `ioctl` submodule below
// (only place unsafe_code is allowed, and only for the libc::ioctl calls).
// The diagnostic path is already using it successfully.
//
// When implementing for real:
// - Use the open fd from RealXdna2Device::as_fd()
// - Build proper command buffers (often in BOs of type AMDXDNA_BO_CMD)
// - Handle the UMQ (user mode queue) and doorbells for low-latency submission
// - Stay lightweight: minimal dependencies, direct use of the mainline uAPI only.

/// Minimal definition needed for the GET_INFO ioctl request constant and calls.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct amdxdna_drm_get_info {
    pub param: u32,
    pub buffer_size: u32,
    pub buffer: u64,
}

#[cfg(feature = "amd-xdna2")]
#[allow(unsafe_code, unsafe_op_in_unsafe_fn, clippy::all)]
mod ioctl {
    //! Narrow module containing the actual (unsafe) ioctl calls against the amdxdna driver.
    //! This is the only place in the crate where unsafe code for ioctls is allowed.
    //!
    //! Goal: Keep the direct driver implementation as small and auditable as possible.

    use std::os::unix::io::RawFd;

    use libc::{c_int, c_ulong, ioctl as libc_ioctl};

    /// Generic ioctl helper.
    ///
    /// Safety: Caller must ensure `arg` points to a valid structure matching the ioctl request,
    /// and that the fd is a valid open amdxdna device.
    pub unsafe fn ioctl<T>(fd: RawFd, request: c_ulong, arg: *mut T) -> std::io::Result<c_int> {
        // The call to the libc ioctl is the unsafe operation.
        let ret = unsafe { libc_ioctl(fd, request, arg.cast::<libc::c_void>()) };
        if ret < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(ret)
        }
    }

    // Proper DRM ioctl request encoding (port of the macros from uapi/drm/drm.h).
    // This keeps us independent of any heavy drm crate while staying correct.
    #[allow(dead_code)]
    const DRM_IOCTL_BASE: u32 = b'd' as u32;
    #[allow(dead_code)]
    const DRM_COMMAND_BASE: u32 = 0x40;

    #[inline]
    #[allow(dead_code, clippy::cast_possible_truncation)]
    pub const fn iowr<T>(nr: u32) -> libc::c_ulong {
        // _IOWR(dir, nr, size) for read+write
        let dir = 3u32;
        let size = core::mem::size_of::<T>() as u32;
        ((dir as libc::c_ulong) << 30)
            | ((size as libc::c_ulong) << 16)
            | ((DRM_IOCTL_BASE as libc::c_ulong) << 8)
            | ((DRM_COMMAND_BASE + nr) as libc::c_ulong)
    }

    // Request for DRM_AMDXDNA_GET_INFO.
    //
    // Computed value for _IOWR('d', 0x40 + 7, sizeof(struct amdxdna_drm_get_info))
    // where sizeof == 16 (param u32 + buffer_size u32 + buffer u64).
    //
    // Derived from:
    //   - amdxdna_drm_ioctl_id enum: DRM_AMDXDNA_GET_INFO = 7
    //   - DRM_COMMAND_BASE = 0x40
    //   - DRM_IOCTL_BASE = 'd' (0x64)
    //   - dir = _IOC_READ | _IOC_WRITE = 3
    //   Result: 0xc010_6447
    //
    // This is the exact value the amdxdna driver expects on this kernel.
    // The iowr::<T>() helper above produces the identical encoding.
    pub const DRM_IOCTL_AMDXDNA_GET_INFO: libc::c_ulong = 0xc010_6447;

    // Request for DRM_AMDXDNA_CREATE_HWCTX.
    //
    // Computed as _IOWR('d', 0x40 + 0, sizeof(struct amdxdna_drm_create_hwctx))
    // sizeof == 56 bytes on this platform.
    //
    // aie2_ctx_init (xdna-driver) requires:
    //   - num_tiles != 0
    //   - a client-owned AMDXDNA_BO_DEV_HEAP already created (64 MiB on AIE2)
    // umq_bo / log_buf_bo are stored but not required for KMQ devices.
    pub const DRM_IOCTL_AMDXDNA_CREATE_HWCTX: libc::c_ulong = 0xc038_6440; // _IOWR for nr=0, size=56

    // Request for DRM_AMDXDNA_DESTROY_HWCTX.
    pub const DRM_IOCTL_AMDXDNA_DESTROY_HWCTX: libc::c_ulong = 0xc008_6441;

    // From amdxdna_drm_ioctl_id (amdxdna_accel.h):
    //   0 CREATE_HWCTX, 1 DESTROY_HWCTX, 2 CONFIG_HWCTX, 3 CREATE_BO,
    //   4 GET_BO_INFO, 5 SYNC_BO, 6 EXEC_CMD, 7 GET_INFO, 8 SET_STATE,
    //   10 GET_ARRAY.
    // There is NO DESTROY_BO command — free BOs with DRM_IOCTL_GEM_CLOSE.
    #[allow(dead_code)]
    pub const DRM_IOCTL_AMDXDNA_CONFIG_HWCTX: libc::c_ulong = 0xc018_6442; // size=24

    // BO management ioctls (CREATE_BO=3, GET_BO_INFO=4, SYNC_BO=5)
    #[allow(dead_code)]
    pub const DRM_IOCTL_AMDXDNA_CREATE_BO: libc::c_ulong = 0xc020_6443; // size=32
    pub const DRM_IOCTL_AMDXDNA_GET_BO_INFO: libc::c_ulong = 0xc030_6444; // size=48
    #[allow(dead_code)]
    pub const DRM_IOCTL_AMDXDNA_SYNC_BO: libc::c_ulong = 0xc018_6445; // size=24

    // Standard DRM GEM close (drm.h): DRM_IOW(0x09, struct drm_gem_close).
    // dir=write-only (1), size=8, type='d', nr=0x09 → 0x40086409.
    // This is the correct way to destroy amdxdna buffer objects.
    pub const DRM_IOCTL_GEM_CLOSE: libc::c_ulong = 0x4008_6409;

    // EXEC_CMD (nr=6)
    //
    // Correctly computed as _IOWR('d', 0x40 + 6, sizeof(struct amdxdna_drm_exec_cmd))
    // where our ported struct is exactly 56 bytes (2*u64 + 2*u32 + 2*u64 + 2*u32 + u64 = 56).
    //
    // Derivation (identical to the GET_INFO and CREATE_HWCTX constants that are live on hardware):
    //   - amdxdna_drm_ioctl_id: DRM_AMDXDNA_EXEC_CMD = 6
    //   - DRM_COMMAND_BASE = 0x40
    //   - DRM_IOCTL_BASE = 'd'
    //   - dir = _IOC_READ | _IOC_WRITE = 3
    //   - size = 0x38 (56)
    //   Result: 0xc038_6446
    //
    // Previous value (0xc028_6446) used an incorrect 40-byte size assumption during
    // struct population. Using the wrong request code would cause the kernel to
    // reject the ioctl before the driver even saw the (still-dummy) payload.
    // This is now exact and matches the iowr::<T> helper formula.
    #[allow(dead_code)]
    pub const DRM_IOCTL_AMDXDNA_EXEC_CMD: libc::c_ulong = 0xc038_6446; // exact: _IOWR('d', 0x46, sizeof(amdxdna_drm_exec_cmd)=56)
}

/// Core pure parser for a BO flags string (comma-separated decimal or 0x-hex).
/// Used by both the env wrapper and tests.
fn parse_bo_flags_list(s: &str) -> Vec<u64> {
    let vals: Vec<u64> = s
        .split(',')
        .filter_map(|p| {
            let p = p.trim();
            if p.is_empty() {
                return None;
            }
            if p.starts_with("0x") || p.starts_with("0X") {
                u64::from_str_radix(&p[2..], 16).ok()
            } else {
                p.parse::<u64>().ok()
            }
        })
        .collect();
    if vals.is_empty() {
        vec![0, 1, 2, 4]
    } else {
        vals
    }
}

/// Parse XDNA2_BO_FLAGS environment variable into a list of u64 flag values.
///
/// Supports comma-separated decimal or 0x-hex numbers.
/// Returns the default [0, 1, 2, 4] when the var is unset or empty/invalid.
/// This enables runtime-driven flag experiments from the probe/example
/// without recompiling (see user's hardware bring-up logs).
fn parse_xdna2_bo_flags_from_env() -> Vec<u64> {
    std::env::var("XDNA2_BO_FLAGS")
        .ok()
        .map_or_else(|| vec![0, 1, 2, 4], |s| parse_bo_flags_list(&s))
}

/// Parse a single u32 from env (decimal or 0x-hex). Returns the provided default if unset/invalid.
fn parse_u32_from_env(key: &str, default: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|s| {
            let s = s.trim();
            if s.starts_with("0x") || s.starts_with("0X") {
                u32::from_str_radix(&s[2..], 16).ok()
            } else {
                s.parse::<u32>().ok()
            }
        })
        .unwrap_or(default)
}

/// Parse a single u64 from env (decimal or 0x-hex). Returns the provided default if unset/invalid.
fn parse_u64_from_env(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|s| {
            let s = s.trim();
            if s.starts_with("0x") || s.starts_with("0X") {
                u64::from_str_radix(&s[2..], 16).ok()
            } else {
                s.parse::<u64>().ok()
            }
        })
        .unwrap_or(default)
}

// Thread-local size preference for `create_hwctx_with_sizes` → Strategy B.
// When set to Some((umq, log)), CREATE_HWCTX Strategy B uses only those BO
// sizes instead of the full bring-up matrix. Cleared after each sized call.
std::thread_local! {
    static HWCTX_SIZE_OVERRIDE: std::cell::Cell<Option<(u64, u64)>> =
        const { std::cell::Cell::new(None) };
}

/// Run `f` with a temporary UMQ/log size override for CREATE_HWCTX Strategy B.
fn with_hwctx_size_override<R>(umq_size: u64, log_size: u64, f: impl FnOnce() -> R) -> R {
    HWCTX_SIZE_OVERRIDE.with(|cell| {
        let prev = cell.replace(Some((umq_size, log_size)));
        let result = f();
        cell.set(prev);
        result
    })
}

/// Build a compact, human-readable summary for a failed CREATE_HWCTX bring-up run.
///
/// Instead of joining every single per-candidate log line (which easily produces
/// 5-15 KB errors that are miserable to read even with truncation in the probe),
/// we produce a short executive summary + key stats. The full raw data is still
/// generated during the run and can be inspected in the source / with a debugger
/// if someone really needs the complete matrix.
fn build_compact_hwctx_failure_summary(attempt_logs: &[String]) -> String {
    let total = attempt_logs.len();
    if total == 0 {
        return "no attempts".to_string();
    }

    // Simple classification based on the strings we already generate.
    let mut create_bo_failures = 0usize;
    let mut create_hwctx_failures = 0usize;
    let mut other = 0usize;
    let mut devheap_busy_seen = false;

    for log in attempt_logs {
        if log.contains("CREATE_BO_FAILED") {
            create_bo_failures += 1;
            if log.contains("busy") || log.contains("EBUSY") || log.contains("os error 16") {
                devheap_busy_seen = true;
            }
        } else if log.contains("CREATE_HWCTX_FAILED") {
            create_hwctx_failures += 1;
        } else {
            other += 1;
        }
    }

    let mut parts = vec![format!(
        "{create_bo_failures} CREATE_BO failures, {create_hwctx_failures} CREATE_HWCTX failures"
    )];

    if devheap_busy_seen {
        parts.push("DevHeap EBUSY seen (skipped repeats)".to_string());
    }
    if other > 0 {
        parts.push(format!("{other} other entries"));
    }

    // Show a couple of representative failures so the user sees the actual errno + diags.
    let head = attempt_logs
        .iter()
        .take(2)
        .cloned()
        .collect::<Vec<_>>()
        .join(" || ");
    let tail = if total > 4 {
        attempt_logs
            .iter()
            .rev()
            .take(2)
            .rev()
            .cloned()
            .collect::<Vec<_>>()
            .join(" || ")
    } else {
        String::new()
    };

    let mut summary = parts.join("; ");
    summary.push_str(". Sample: ");
    summary.push_str(&head);
    if !tail.is_empty() && tail != head {
        summary.push_str(" ... ");
        summary.push_str(&tail);
    }
    let _ = write!(
        summary,
        " ({total} total; full matrix in source for this build)"
    );

    summary
}

/// AIE2 device-heap size required by the amdxdna driver (`AIE2_DEVM_SIZE`).
///
/// CREATE_BO of type `AMDXDNA_BO_DEV_HEAP` must be a multiple of this value.
/// `aie2_ctx_init` refuses CREATE_HWCTX with `-ENOENT` until the client has
/// exactly one such heap BO registered (`client->dev_heap`).
pub const AIE2_DEV_HEAP_SIZE: u64 = 64 * 1024 * 1024;

/// Tracked client DevHeap (handle + optional mmap of the full 64 MiB).
///
/// `aie2_ctx_init` pins the heap and then allocates internal DEV BOs from it.
/// Those allocations require `amdxdna_gem_uva(heap) != INVALID`, which is only
/// set after userspace mmaps the BO (see amdxdna_gem.c / amdxdna_hmm_register).
#[derive(Debug)]
struct DevHeapState {
    handle: u32,
    /// Kernel virtual mapping of the heap (kept alive for the fd lifetime).
    #[allow(dead_code)]
    map_ptr: *mut libc::c_void,
    #[allow(dead_code)]
    map_len: usize,
}

// SAFETY: DevHeapState is only mutated under a Mutex; the mmap is process-local.
#[allow(unsafe_code)]
unsafe impl Send for DevHeapState {}
// SAFETY: same as Send — exclusive access via Mutex.
#[allow(unsafe_code)]
unsafe impl Sync for DevHeapState {}

/// The real (non-stub) lightweight direct submission implementation.
///
/// Owns the open accel fd and tracks the client-scoped DevHeap BO that the
/// kernel requires before CREATE_HWCTX can succeed on KMQ/AIE2 devices.
#[derive(Debug)]
#[allow(dead_code)] // Real implementation scaffolding for the lightweight direct path
pub struct RealDirectSubmission {
    /// The raw fd to the accel device (e.g. /dev/accel/accel0).
    fd: RawFd,
    /// Client-owned DevHeap (handle + mmap). Interior mutability so
    /// `create_hwctx` can ensure the heap once per submission object.
    dev_heap: std::sync::Mutex<Option<DevHeapState>>,
}

impl RealDirectSubmission {
    /// Create a real direct submission context for the given open fd.
    /// All future real ioctl work will go through this fd.
    pub fn new(fd: RawFd) -> Self {
        Self {
            fd,
            dev_heap: std::sync::Mutex::new(None),
        }
    }

    /// Return the tracked DevHeap handle (0 if none yet).
    pub fn dev_heap_handle(&self) -> u32 {
        self.dev_heap
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|s| s.handle))
            .unwrap_or(0)
    }

    /// Ensure the client has a DevHeap BO registered **and mmap'd**.
    ///
    /// Per `amdxdna_drm_create_dev_heap_bo` / `aie2_ctx_init` in xdna-driver:
    /// - Type must be `AMDXDNA_BO_DEV_HEAP`
    /// - Size must be a multiple of `AIE2_DEVM_SIZE` (64 MiB)
    /// - Only one DevHeap is allowed per client (second create → EBUSY)
    /// - Heap UVA must be valid (requires mmap) before CREATE_HWCTX can
    ///   allocate internal DEV command buffers from the heap
    /// - The accel fd **must** be opened `O_RDWR`. A read-only fd makes
    ///   `mmap(..., PROT_READ|PROT_WRITE, ...)` return `EPERM` even when
    ///   CREATE_BO + GET_BO_INFO succeed (root-caused 2026-07-13 on 1022:17f0).
    ///
    /// Without this BO, CREATE_HWCTX fails with ENOENT
    /// ("The client dev heap object not exist").
    /// With the BO but without mmap, CREATE_HWCTX fails later with EINVAL
    /// ("Invalid dev heap userptr").
    pub fn ensure_dev_heap(&self) -> Result<BufferHandle, DirectError> {
        {
            let guard = self
                .dev_heap
                .lock()
                .map_err(|_| DirectError::DriverError(-1))?;
            if let Some(ref state) = *guard {
                return Ok(BufferHandle(state.handle));
            }
        }

        // flags must be 0 (MBZ per amdxdna_drm_create_bo).
        let heap = match self.create_buffer_with_flags(AIE2_DEV_HEAP_SIZE, BufferType::DevHeap, 0) {
            Ok(h) => h,
            Err(e) => {
                // EBUSY means another RealDirectSubmission (or earlier call)
                // already registered the single client DevHeap on this fd.
                // The driver still has it; we cannot rediscover the handle, but
                // CREATE_HWCTX only needs client->dev_heap to be non-NULL.
                // We still need a valid UVA though — without the handle we
                // cannot mmap. Surface a clear error so the caller reuses one
                // submission object (or reopens the fd).
                let msg = format!("{e}");
                if msg.contains("busy") || msg.contains("os error 16") {
                    return Err(DirectError::IoctlFailed(
                        "DevHeap already exists on this fd (EBUSY) but this RealDirectSubmission \
                         instance does not own the handle/mmap. Reuse one submission object per fd, \
                         or reopen /dev/accel/accelN. Original: "
                            .to_string()
                            + &msg,
                    ));
                }
                return Err(e);
            }
        };

        // GET_BO_INFO → mmap so amdxdna_gem_uva(heap) becomes valid.
        let info = self.get_bo_info(self.fd, heap.0)?;
        if info.map_offset == 0 || info.map_offset == u64::MAX {
            let _ = self.destroy_buffer(self.fd, heap.0);
            return Err(DirectError::IoctlFailed(format!(
                "DevHeap handle={} has invalid map_offset={:#x}; cannot mmap for UVA",
                heap.0, info.map_offset
            )));
        }

        #[allow(clippy::cast_possible_truncation)]
        let map_len = AIE2_DEV_HEAP_SIZE as usize;
        #[allow(
            unsafe_code,
            clippy::cast_possible_truncation,
            clippy::cast_possible_wrap
        )]
        let map_ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                map_len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                self.fd,
                // DRM fake offset is a full u64; cast for mmap off_t.
                info.map_offset as libc::off_t,
            )
        };
        if map_ptr == libc::MAP_FAILED {
            let err = std::io::Error::last_os_error();
            let _ = self.destroy_buffer(self.fd, heap.0);
            let hint = if err.raw_os_error() == Some(libc::EACCES)
                || err.raw_os_error() == Some(libc::EPERM)
            {
                " (hint: open /dev/accel O_RDWR — O_RDONLY yields EPERM on PROT_WRITE mmap)"
            } else {
                ""
            };
            return Err(DirectError::IoctlFailed(format!(
                "mmap DevHeap handle={} map_offset={:#x} size={map_len} failed: {err}{hint}",
                heap.0, info.map_offset
            )));
        }

        if let Ok(mut guard) = self.dev_heap.lock() {
            *guard = Some(DevHeapState {
                handle: heap.0,
                map_ptr,
                map_len,
            });
        }
        Ok(heap)
    }

    /// Creates a UMQ (User Mode Queue) BO. Optional on KMQ devices (can pass 0).
    pub fn create_umq_bo(
        &self,
        size: u64,
        ty: BufferType,
        flags: u64,
    ) -> Result<BufferHandle, DirectError> {
        self.create_buffer_with_flags(size, ty, flags)
    }

    /// Creates a log buffer BO. Optional on KMQ devices (can pass 0).
    pub fn create_log_buffer_bo(
        &self,
        size: u64,
        ty: BufferType,
        flags: u64,
    ) -> Result<BufferHandle, DirectError> {
        self.create_buffer_with_flags(size, ty, flags)
    }

    /// Internal helper: real CREATE_BO with caller-supplied flags.
    /// Public trait `create_buffer` always uses flags=0 (header: MBZ).
    #[allow(unsafe_code, clippy::all)]
    fn create_buffer_with_flags(
        &self,
        size: u64,
        ty: BufferType,
        flags: u64,
    ) -> Result<BufferHandle, DirectError> {
        #[cfg(feature = "amd-xdna2")]
        {
            let mut req = amdxdna_drm_create_bo {
                flags,
                vaddr: 0,
                size,
                ty: ty as u32,
                handle: 0,
            };

            let ret =
                unsafe { ioctl::ioctl(self.fd, ioctl::DRM_IOCTL_AMDXDNA_CREATE_BO, &raw mut req) };

            match ret {
                Ok(_) => Ok(BufferHandle(req.handle)),
                Err(e) => Err(DirectError::IoctlFailed(format!(
                    "CREATE_BO failed (size={size}, ty={ty:?}, flags=0x{flags:x}): {e}"
                ))),
            }
        }

        #[cfg(not(feature = "amd-xdna2"))]
        {
            let _ = (size, ty, flags);
            Err(DirectError::Unsupported(
                "create_buffer requires the amd-xdna2 feature".to_string(),
            ))
        }
    }
}

// We implement DirectSubmission for RealDirectSubmission as a "real shape" stub.
// It captures the fd and uses the ported structs, but still returns clear
// "not yet implemented" errors. This makes the transition to real ioctls
// much smaller when we are ready.
impl DirectSubmission for RealDirectSubmission {
    fn create_hwctx_with_sizes(
        &self,
        fd: RawFd,
        num_tiles: u32,
        mem_size: u32,
        umq_size: u64,
        log_size: u64,
        qos: Option<&QosInfo>,
    ) -> Result<HardwareContext, DirectError> {
        // Pin Strategy B to the caller sizes (see HWCTX_SIZE_OVERRIDE).
        with_hwctx_size_override(umq_size, log_size, || {
            self.create_hwctx(fd, num_tiles, mem_size, qos)
        })
    }

    #[allow(unsafe_code, clippy::all, unused_variables, clippy::too_many_lines)]
    fn create_hwctx(
        &self,
        _fd: RawFd,
        num_tiles: u32,
        mem_size: u32,
        qos: Option<&QosInfo>,
    ) -> Result<HardwareContext, DirectError> {
        // Real CREATE_HWCTX for the lightweight direct path.
        //
        // Kernel requirements (aie2_ctx_init / amdxdna_accel.h):
        //   1. num_tiles != 0
        //   2. Client must already own exactly one AMDXDNA_BO_DEV_HEAP
        //      (size multiple of AIE2_DEVM_SIZE = 64 MiB)
        //   3. On KMQ devices, umq_bo / log_buf_bo are optional (may be 0)
        //   4. CREATE_BO.flags is MBZ (must be 0) per the public header
        //   5. BOs are freed with DRM_IOCTL_GEM_CLOSE (no DESTROY_BO ioctl)
        //
        // Historical bring-up mistakes this path now corrects:
        //   - Using DevHeap as the UMQ type at 4 KiB (wrong; DevHeap is the
        //     separate 64 MiB client heap, not the queue)
        //   - Never calling ensure_dev_heap before CREATE_HWCTX
        //   - Routing get_bo_info through GET_INFO param 4 (sensors) instead of
        //     DRM_IOCTL_AMDXDNA_GET_BO_INFO
        //   - Trying fake DESTROY_BO nrs instead of GEM_CLOSE
        //
        // Env overrides still available for field experiments:
        //   XDNA2_BO_FLAGS, XDNA2_HWCTX_EXT_FLAGS, XDNA2_HWCTX_MAX_OPC,
        //   XDNA2_HWCTX_UMQ_DOORBELL, XDNA2_HWCTX_SKIP_HEAP=1

        let qos_info = qos.copied().unwrap_or_default();

        #[cfg(feature = "amd-xdna2")]
        {
            let mut attempt_logs: Vec<String> = Vec::new();

            // --- Step 0: ensure the required 64 MiB DevHeap ---------------------
            let skip_heap = std::env::var("XDNA2_HWCTX_SKIP_HEAP")
                .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));

            let heap_handle = if skip_heap {
                attempt_logs.push("DEV_HEAP_SKIPPED (XDNA2_HWCTX_SKIP_HEAP=1)".to_string());
                0u32
            } else {
                match self.ensure_dev_heap() {
                    Ok(h) => {
                        let info = self.get_bo_info(self.fd, h.0).ok();
                        attempt_logs.push(format!(
                            "DEV_HEAP_OK handle={} size={} info={info:?}",
                            h.0, AIE2_DEV_HEAP_SIZE
                        ));
                        h.0
                    }
                    Err(e) => {
                        // EBUSY: another RealDirectSubmission on this same client fd
                        // already created the single DevHeap. We do not munmap on drop,
                        // so the prior mmap (and thus heap UVA) should still be valid.
                        // aie2_ctx_init only needs client->dev_heap non-NULL + valid UVA.
                        let msg = format!("{e}");
                        if msg.contains("busy")
                            || msg.contains("os error 16")
                            || msg.contains("EBUSY")
                        {
                            attempt_logs.push(format!(
                                            "DEV_HEAP_EBUSY_REUSE (prior heap+mmap assumed valid on this fd): {msg}"
                                        ));
                            0u32
                        } else {
                            return Err(DirectError::IoctlFailed(format!(
                                "CREATE_HWCTX blocked: ensure_dev_heap failed (64 MiB AMDXDNA_BO_DEV_HEAP required by aie2_ctx_init): {e}"
                            )));
                        }
                    }
                }
            };

            let ext_flags = parse_u64_from_env("XDNA2_HWCTX_EXT_FLAGS", 0);
            let max_opc = parse_u32_from_env("XDNA2_HWCTX_MAX_OPC", 0);
            let umq_doorbell = parse_u32_from_env("XDNA2_HWCTX_UMQ_DOORBELL", 0);

            // Live diagnostics shared across attempts.
            let extra_diag = {
                #[allow(clippy::map_unwrap_or)]
                let pwr = self.get_power_mode().map(|p| p.power_mode).unwrap_or(255);
                #[allow(clippy::map_unwrap_or)]
                let aie = self
                    .get_aie_version()
                    .map(|a| format!("{}.{}", a.major, a.minor))
                    .unwrap_or_else(|_| "err".to_string());
                #[allow(clippy::map_unwrap_or)]
                let res = self
                    .get_resource_info()
                    .map(|r| format!("tops_max={} clk_max={}", r.npu_tops_max, r.npu_clk_max))
                    .unwrap_or_else(|_| "res=err".to_string());
                format!("heap={heap_handle} pwr={pwr} aie={aie} {res}")
            };

            // Helper: issue CREATE_HWCTX with the given umq/log handles.
            let try_create = |umq_bo: u32,
                              log_buf_bo: u32,
                              label: &str,
                              attempt_logs: &mut Vec<String>|
             -> Result<HardwareContext, ()> {
                // CRITICAL (amdxdna_drm_create_hwctx_ioctl in xdna-driver):
                //   if (args->ext || args->ext_flags) return -EINVAL;
                //   copy_from_user(&ctx->qos, u64_to_user_ptr(args->qos_p), ...)
                // ALWAYS runs — qos_p MUST be a valid userspace pointer. Passing 0
                // yields -EFAULT (or EINVAL on some kernels). Official clients always
                // supply a qos struct; zeroed defaults are fine (is_valid_qos_dpm_params
                // then falls back to max/low DPM).
                // XDNA2_HWCTX_QOS_NULL=1 reverts to the old null experiment.
                let force_null_qos = std::env::var("XDNA2_HWCTX_QOS_NULL")
                    .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
                let qos_p = if force_null_qos {
                    0u64
                } else {
                    &raw const qos_info as u64
                };

                let mut req = amdxdna_drm_create_hwctx {
                    ext: 0,
                    ext_flags,
                    qos_p,
                    umq_bo,
                    log_buf_bo,
                    max_opc,
                    num_tiles,
                    mem_size,
                    umq_doorbell,
                    handle: 0,
                    syncobj_handle: 0,
                };

                let ret = unsafe {
                    ioctl::ioctl(self.fd, ioctl::DRM_IOCTL_AMDXDNA_CREATE_HWCTX, &raw mut req)
                };

                match ret {
                    Ok(_) => {
                        attempt_logs.push(format!(
                            "CREATE_HWCTX_OK label={label} handle={} umq={umq_bo} log={log_buf_bo} doorbell={} syncobj={} {extra_diag}",
                            req.handle, req.umq_doorbell, req.syncobj_handle
                        ));
                        Ok(HardwareContext {
                            handle: req.handle,
                            num_tiles,
                            mem_size,
                            umq_bo,
                            log_buf_bo,
                        })
                    }
                    Err(e) => {
                        attempt_logs.push(format!(
                            "CREATE_HWCTX_FAILED label={label} tiles={num_tiles} mem={mem_size} \
                             umq={umq_bo} log={log_buf_bo} max_opc={max_opc} ext_flags=0x{ext_flags:x} \
                             qos_p=0x{qos_p:x}: {e}. {extra_diag}"
                        ));
                        Err(())
                    }
                }
            };

            // --- Strategy A: KMQ zero-BO path (umq/log optional) --------------
            if let Ok(ctx) = try_create(0, 0, "kmq_zero_bo", &mut attempt_logs) {
                return Ok(ctx);
            }

            // --- Strategy B: Shmem UMQ + Shmem log (flags MBZ by default) -----
            // Do NOT use DevHeap here — that type is reserved for the single
            // client heap created in ensure_dev_heap above.
            //
            // Size policy:
            //   - create_hwctx_with_sizes → only the caller-requested UMQ/log sizes
            //   - create_hwctx (default)  → full bring-up matrix (4–64 KiB UMQ, 128 KiB log)
            //   - XDNA2_HWCTX_UMQ / XDNA2_HWCTX_LOG env can pin sizes without API change
            let env_umq = std::env::var("XDNA2_HWCTX_UMQ").ok().and_then(|s| {
                let s = s.trim();
                if s.starts_with("0x") || s.starts_with("0X") {
                    u64::from_str_radix(&s[2..], 16).ok()
                } else {
                    s.parse::<u64>().ok()
                }
            });
            let env_log = std::env::var("XDNA2_HWCTX_LOG").ok().and_then(|s| {
                let s = s.trim();
                if s.starts_with("0x") || s.starts_with("0X") {
                    u64::from_str_radix(&s[2..], 16).ok()
                } else {
                    s.parse::<u64>().ok()
                }
            });
            let api_override = HWCTX_SIZE_OVERRIDE.with(std::cell::Cell::get);
            let (log_size, umq_owned): (u64, Vec<u64>) = if let Some((u, l)) = api_override {
                attempt_logs.push(format!("SIZE_POLICY=api_override umq={u} log={l}"));
                (l.max(4096), vec![u.max(4096)])
            } else if let (Some(u), Some(l)) = (env_umq, env_log) {
                attempt_logs.push(format!(
                    "SIZE_POLICY=env XDNA2_HWCTX_UMQ={u} XDNA2_HWCTX_LOG={l}"
                ));
                (l.max(4096), vec![u.max(4096)])
            } else if let Some(u) = env_umq {
                attempt_logs.push(format!("SIZE_POLICY=env_umq_only umq={u} log=131072"));
                (128 * 1024, vec![u.max(4096)])
            } else if let Some(l) = env_log {
                attempt_logs.push(format!("SIZE_POLICY=env_log_only log={l} umq=matrix"));
                (
                    l.max(4096),
                    vec![4 * 1024, 8 * 1024, 16 * 1024, 32 * 1024, 64 * 1024],
                )
            } else {
                attempt_logs.push("SIZE_POLICY=probe_matrix".to_string());
                (
                    128 * 1024,
                    vec![4 * 1024, 8 * 1024, 16 * 1024, 32 * 1024, 64 * 1024],
                )
            };
            let umq_candidates: &[u64] = &umq_owned;

            // Header says CREATE_BO.flags is MBZ. Default to [0] only; env can
            // still expand the matrix for experiments (XDNA2_BO_FLAGS).
            let flags_candidates: Vec<u64> = if std::env::var("XDNA2_BO_FLAGS").is_ok() {
                parse_xdna2_bo_flags_from_env()
            } else {
                vec![0]
            };
            let sync_directions: &[u32] = &[0, 1]; // TO_DEVICE, FROM_DEVICE per header

            for &umq_size in umq_candidates {
                for &bo_flags in &flags_candidates {
                    let umq_bo = match self.create_umq_bo(umq_size, BufferType::Shmem, bo_flags) {
                        Ok(h) => h,
                        Err(e) => {
                            attempt_logs.push(format!(
                                "UMQ_CREATE_BO_FAILED size={umq_size} ty=Shmem flags=0x{bo_flags:x}: {e}"
                            ));
                            continue;
                        }
                    };
                    let log_buf_bo =
                        match self.create_log_buffer_bo(log_size, BufferType::Shmem, bo_flags) {
                            Ok(h) => h,
                            Err(e) => {
                                let destroy_res = self.destroy_buffer(self.fd, umq_bo.0);
                                attempt_logs.push(format!(
                                    "LOG_CREATE_BO_FAILED: {e}; destroy_umq={destroy_res:?}"
                                ));
                                continue;
                            }
                        };

                    for &sync_dir in sync_directions {
                        let umq_info_after_create = self.get_bo_info(self.fd, umq_bo.0).ok();
                        let log_info_after_create = self.get_bo_info(self.fd, log_buf_bo.0).ok();

                        let umq_sync_ok = self
                            .sync_buffer(self.fd, umq_bo, sync_dir, 0, umq_size)
                            .is_ok();
                        let log_sync_ok = self
                            .sync_buffer(self.fd, log_buf_bo, sync_dir, 0, log_size)
                            .is_ok();

                        let umq_info_after_sync = self.get_bo_info(self.fd, umq_bo.0).ok();
                        let log_info_after_sync = self.get_bo_info(self.fd, log_buf_bo.0).ok();

                        // Best-effort mmap when map_offset is non-zero (now that
                        // GET_BO_INFO uses the correct ioctl).
                        if let Some(info) = umq_info_after_sync {
                            if info.map_offset != 0 {
                                #[allow(
                                    clippy::cast_possible_truncation,
                                    clippy::cast_possible_wrap,
                                    clippy::ignored_unit_patterns
                                )]
                                let () = unsafe {
                                    let ptr = libc::mmap(
                                        std::ptr::null_mut(),
                                        umq_size as libc::size_t,
                                        libc::PROT_READ | libc::PROT_WRITE,
                                        libc::MAP_SHARED,
                                        self.fd,
                                        info.map_offset as libc::off_t,
                                    );
                                    if ptr != libc::MAP_FAILED {
                                        let _ = libc::munmap(ptr, umq_size as libc::size_t);
                                    }
                                };
                            }
                        }

                        let label =
                            format!("shmem_umq={umq_size}/flags=0x{bo_flags:x}/sync={sync_dir}");
                        match try_create(umq_bo.0, log_buf_bo.0, &label, &mut attempt_logs) {
                            Ok(ctx) => return Ok(ctx),
                            Err(()) => {
                                attempt_logs.push(format!(
                                    "  diag umq_create={umq_info_after_create:?} umq_sync_ok={umq_sync_ok} umq_sync={umq_info_after_sync:?} \
                                     log_create={log_info_after_create:?} log_sync_ok={log_sync_ok} log_sync={log_info_after_sync:?}"
                                ));
                            }
                        }
                    }

                    // Cleanup UMQ/log for this size/flags before next candidate.
                    // Do NOT close the DevHeap — it stays for the client lifetime.
                    let _ = self.destroy_buffer(self.fd, umq_bo.0);
                    let _ = self.destroy_buffer(self.fd, log_buf_bo.0);
                }
            }

            let summary = if attempt_logs.is_empty() {
                "no attempts (unexpected)".to_string()
            } else {
                build_compact_hwctx_failure_summary(&attempt_logs)
            };
            Err(DirectError::IoctlFailed(format!(
                "CREATE_HWCTX failed after KMQ-zero + {} Shmem UMQ sizes x {} flags x {} sync_dirs ({} log lines): {}",
                umq_candidates.len(),
                flags_candidates.len(),
                sync_directions.len(),
                attempt_logs.len(),
                summary
            )))
        }

        #[cfg(not(feature = "amd-xdna2"))]
        {
            let _ = (num_tiles, mem_size, qos);
            Err(DirectError::Unsupported(
                "create_hwctx requires the amd-xdna2 feature".to_string(),
            ))
        }
    }

    #[allow(unsafe_code)]
    fn destroy_hwctx(&self, _fd: RawFd, handle: u32) -> Result<(), DirectError> {
        #[cfg(feature = "amd-xdna2")]
        {
            let mut req = amdxdna_drm_destroy_hwctx { handle, pad: 0 };

            let ret = unsafe {
                ioctl::ioctl(
                    self.fd,
                    ioctl::DRM_IOCTL_AMDXDNA_DESTROY_HWCTX,
                    &raw mut req,
                )
            };

            match ret {
                Ok(_) => Ok(()),
                Err(e) => Err(DirectError::IoctlFailed(format!(
                    "DESTROY_HWCTX failed for handle {handle}: {e}"
                ))),
            }
        }

        #[cfg(not(feature = "amd-xdna2"))]
        {
            let _ = handle;
            Ok(())
        }
    }

    #[allow(unsafe_code, clippy::all)]
    fn create_buffer(
        &self,
        _fd: RawFd,
        size: u64,
        ty: BufferType,
    ) -> Result<BufferHandle, DirectError> {
        #[cfg(feature = "amd-xdna2")]
        {
            let mut req = amdxdna_drm_create_bo {
                flags: 0,
                vaddr: 0,
                size,
                ty: ty as u32,
                handle: 0,
            };

            let ret =
                unsafe { ioctl::ioctl(self.fd, ioctl::DRM_IOCTL_AMDXDNA_CREATE_BO, &raw mut req) };

            match ret {
                Ok(_) => Ok(BufferHandle(req.handle)),
                Err(e) => Err(DirectError::IoctlFailed(format!(
                    "CREATE_BO failed (size={size}, ty={ty:?}): {e}"
                ))),
            }
        }

        #[cfg(not(feature = "amd-xdna2"))]
        {
            let _ = (size, ty);
            Err(DirectError::Unsupported(
                "create_buffer requires the amd-xdna2 feature".to_string(),
            ))
        }
    }

    #[allow(unsafe_code)]
    fn destroy_buffer(&self, _fd: RawFd, handle: u32) -> Result<(), DirectError> {
        #[cfg(feature = "amd-xdna2")]
        {
            // amdxdna has no DESTROY_BO ioctl. BOs are standard DRM GEM objects
            // and must be released with DRM_IOCTL_GEM_CLOSE (drm.h).
            // Earlier bring-up incorrectly tried amdxdna command nrs (including
            // nr=2 which is CONFIG_HWCTX) — those always returned EINVAL and left
            // BOs alive, causing DevHeap EBUSY on subsequent creates.
            let mut req = drm_gem_close { handle, pad: 0 };

            let ret = unsafe { ioctl::ioctl(self.fd, ioctl::DRM_IOCTL_GEM_CLOSE, &raw mut req) };

            match ret {
                Ok(_) => Ok(()),
                Err(e) => Err(DirectError::IoctlFailed(format!(
                    "GEM_CLOSE failed for handle={handle} (request=0x{:x}): {e}",
                    ioctl::DRM_IOCTL_GEM_CLOSE
                ))),
            }
        }

        #[cfg(not(feature = "amd-xdna2"))]
        {
            let _ = handle;
            Err(DirectError::Unsupported(
                "destroy_buffer requires the amd-xdna2 feature".to_string(),
            ))
        }
    }

    #[allow(unsafe_code)]
    fn submit_command(&self, _fd: RawFd, cmd: &Command) -> Result<u64, DirectError> {
        #[cfg(feature = "amd-xdna2")]
        {
            // NEVER AGAIN (2026-07-13): un-gated EXEC_CMD with scaffold payload
            // NULL-deref'd aie2_cmdlist_fill_slot and wedged open(). Double gate
            // in exec_cmd_is_enabled() before any ioctl.
            if !exec_cmd_is_enabled() {
                return Err(DirectError::Unsupported(
                    exec_cmd_block_reason().to_string(),
                ));
            }

            // Remaining hard unknown: real firmware txn / cmdlist payload layout.
            // Even when enabled, only known-good payloads should be submitted.

            let mut req = amdxdna_drm_exec_cmd {
                ext: 0,
                ext_flags: 0,
                hwctx: cmd.hwctx_handle,
                ty: cmd.ty as u32,
                cmd_handles: cmd.cmd_bo_handle.map_or(0, u64::from),
                args: 0,
                cmd_count: u32::from(cmd.cmd_bo_handle.is_some()),
                arg_count: 0,
                seq: 0,
            };

            let ret =
                unsafe { ioctl::ioctl(self.fd, ioctl::DRM_IOCTL_AMDXDNA_EXEC_CMD, &raw mut req) };

            match ret {
                Ok(_) => Ok(req.seq),
                Err(e) => Err(DirectError::IoctlFailed(format!(
                    "EXEC_CMD failed (hwctx={}, ty={:?}, cmd_bo_handle={:?}, payload_len={}): {e}. \
                     Note: real command payload (txn ops / control code inside AMDXDNA_BO_CMD BO) \
                     is not yet implemented. The EXEC_CMD ioctl request code is now exactly \
                     0xc038_6446. This is currently the main remaining gap for actual model \
                     execution on real XDNA2 hardware (we have the full ioctl path + rich \
                     hwctx/BO scaffolding + correct request codes + ability to point at real \
                     command BOs, but the firmware command format / txn encoding is the unknown).",
                    cmd.hwctx_handle,
                    cmd.ty,
                    cmd.cmd_bo_handle,
                    cmd.payload.len()
                ))),
            }
        }

        #[cfg(not(feature = "amd-xdna2"))]
        {
            let _ = cmd;
            Err(DirectError::Unsupported(
                "submit_command requires the amd-xdna2 feature".to_string(),
            ))
        }
    }

    #[allow(unsafe_code)]
    fn sync_buffer(
        &self,
        _fd: RawFd,
        handle: BufferHandle,
        direction: u32,
        offset: u64,
        size: u64,
    ) -> Result<(), DirectError> {
        #[cfg(feature = "amd-xdna2")]
        {
            let mut req = amdxdna_drm_sync_bo {
                handle: handle.0,
                direction,
                offset,
                size,
            };

            let ret =
                unsafe { ioctl::ioctl(self.fd, ioctl::DRM_IOCTL_AMDXDNA_SYNC_BO, &raw mut req) };

            match ret {
                Ok(_) => Ok(()),
                Err(e) => Err(DirectError::IoctlFailed(format!(
                    "SYNC_BO failed (handle={}, dir={}, off={}, size={}): {e}",
                    handle.0, direction, offset, size
                ))),
            }
        }

        #[cfg(not(feature = "amd-xdna2"))]
        {
            let _ = (handle, direction, offset, size);
            Ok(())
        }
    }

    #[allow(unsafe_code, clippy::all)]
    fn get_info(&self, _fd: RawFd, param: u32, buffer: &mut [u8]) -> Result<(), DirectError> {
        // First real driver interaction in the lightweight direct path.
        // We start with read-only GET_INFO queries (safe and very useful on real hardware).

        #[cfg(feature = "amd-xdna2")]
        {
            let mut req = amdxdna_drm_get_info {
                param,
                #[allow(clippy::cast_possible_truncation)]
                buffer_size: u32::try_from(buffer.len()).unwrap_or(u32::MAX),
                buffer: buffer.as_mut_ptr().cast::<libc::c_void>() as u64,
            };

            // Safety: valid pointer + size, kernel only writes into the provided buffer.
            let ret =
                unsafe { ioctl::ioctl(self.fd, ioctl::DRM_IOCTL_AMDXDNA_GET_INFO, &raw mut req) };

            match ret {
                Ok(_) => Ok(()),
                Err(e) => {
                    // Bring-up aid: include the exact request we tried so the user
                    // can easily experiment with nearby values on real hardware.
                    Err(DirectError::IoctlFailed(format!(
                        "GET_INFO param {param} failed (request=0x{:x}): {e}",
                        ioctl::DRM_IOCTL_AMDXDNA_GET_INFO
                    )))
                }
            }
        }

        #[cfg(not(feature = "amd-xdna2"))]
        {
            let _ = (param, buffer);
            Err(DirectError::Unsupported(
                "direct get_info requires the amd-xdna2 feature".to_string(),
            ))
        }
    }

    /// High-level typed query for firmware version.
    /// This performs a real GET_INFO ioctl when running on hardware with the real arm.
    fn get_firmware_version(&self) -> Result<amdxdna_drm_query_firmware_version, DirectError> {
        const PARAM_FIRMWARE_VERSION: u32 = 8; // From the header enum

        let mut ver = amdxdna_drm_query_firmware_version {
            major: 0,
            minor: 0,
            patch: 0,
            build: 0,
        };

        // Prepare buffer view over the struct.
        #[allow(
            unsafe_code,
            clippy::borrow_as_ptr,
            clippy::ref_as_ptr,
            clippy::ptr_as_ptr
        )]
        let buf = unsafe {
            std::slice::from_raw_parts_mut(
                &mut ver as *mut _ as *mut u8,
                std::mem::size_of::<amdxdna_drm_query_firmware_version>(),
            )
        };

        self.get_info(self.fd, PARAM_FIRMWARE_VERSION, buf)?;
        Ok(ver)
    }

    fn get_power_mode(&self) -> Result<amdxdna_drm_get_power_mode, DirectError> {
        // From amdxdna_drm_get_param: immediately after FIRMWARE_VERSION=8
        const PARAM_POWER_MODE: u32 = 9;

        let mut mode = amdxdna_drm_get_power_mode {
            power_mode: 0,
            pad: [0; 7],
        };

        #[allow(
            unsafe_code,
            clippy::all,
            clippy::borrow_as_ptr,
            clippy::ref_as_ptr,
            clippy::ptr_as_ptr
        )]
        let buf = unsafe {
            std::slice::from_raw_parts_mut(
                &mut mode as *mut _ as *mut u8,
                std::mem::size_of::<amdxdna_drm_get_power_mode>(),
            )
        };

        self.get_info(self.fd, PARAM_POWER_MODE, buf)?;
        Ok(mode)
    }

    fn get_resource_info(&self) -> Result<amdxdna_drm_get_resource_info, DirectError> {
        // From amdxdna_drm_get_param enum in the kernel header:
        // FIRMWARE_VERSION=8, GET_POWER_MODE=9, QUERY_TELEMETRY=10, GET_FORCE_PREEMPT=11, QUERY_RESOURCE_INFO=12
        const PARAM_RESOURCE_INFO: u32 = 12;

        let mut info = amdxdna_drm_get_resource_info {
            npu_clk_max: 0,
            npu_tops_max: 0,
            npu_task_max: 0,
            npu_tops_curr: 0,
            npu_task_curr: 0,
        };

        #[allow(
            unsafe_code,
            clippy::all,
            clippy::borrow_as_ptr,
            clippy::ref_as_ptr,
            clippy::ptr_as_ptr
        )]
        let buf = unsafe {
            std::slice::from_raw_parts_mut(
                &mut info as *mut _ as *mut u8,
                std::mem::size_of::<amdxdna_drm_get_resource_info>(),
            )
        };

        self.get_info(self.fd, PARAM_RESOURCE_INFO, buf)?;
        Ok(info)
    }

    /// Query sensor data (temperature, power, etc.).
    /// Param 4 = QUERY_SENSORS in the kernel header.
    fn get_sensors(&self) -> Result<amdxdna_drm_query_sensor, DirectError> {
        const PARAM_SENSORS: u32 = 4;

        let mut sensor = amdxdna_drm_query_sensor {
            label: [0; 64],
            input: 0,
            max: 0,
            average: 0,
            highest: 0,
            status: [0; 64],
            units: [0; 16],
            unitm: 0,
            ty: 0,
            pad: [0; 6],
        };

        #[allow(
            unsafe_code,
            clippy::all,
            clippy::borrow_as_ptr,
            clippy::ref_as_ptr,
            clippy::ptr_as_ptr
        )]
        let buf = unsafe {
            std::slice::from_raw_parts_mut(
                &mut sensor as *mut _ as *mut u8,
                std::mem::size_of::<amdxdna_drm_query_sensor>(),
            )
        };

        self.get_info(self.fd, PARAM_SENSORS, buf)?;
        Ok(sensor)
    }

    /// Query AIE version (major/minor) via GET_INFO.
    fn get_aie_version(&self) -> Result<amdxdna_drm_query_aie_version, DirectError> {
        // Param 2 = QUERY_AIE_VERSION (from amdxdna_drm_get_param enum ordering)
        const PARAM_AIE_VERSION: u32 = 2;

        let mut ver = amdxdna_drm_query_aie_version { major: 0, minor: 0 };

        #[allow(unsafe_code, clippy::all, clippy::borrow_as_ptr, clippy::ptr_as_ptr)]
        let buf = unsafe {
            std::slice::from_raw_parts_mut(
                &raw mut ver as *mut u8,
                std::mem::size_of::<amdxdna_drm_query_aie_version>(),
            )
        };

        self.get_info(self.fd, PARAM_AIE_VERSION, buf)?;
        Ok(ver)
    }

    /// Get detailed information about a buffer object (map offset, XDNA device address, etc.).
    ///
    /// Uses the dedicated `DRM_IOCTL_AMDXDNA_GET_BO_INFO` ioctl (nr=4), **not**
    /// `GET_INFO` param 4 (which is `QUERY_SENSORS`). Earlier code incorrectly
    /// routed through GET_INFO, so map_offset/xdna_addr always looked dead.
    #[allow(unsafe_code, clippy::all)]
    fn get_bo_info(&self, _fd: RawFd, handle: u32) -> Result<amdxdna_drm_get_bo_info, DirectError> {
        #[cfg(feature = "amd-xdna2")]
        {
            let mut info = amdxdna_drm_get_bo_info {
                ext: 0,
                ext_flags: 0,
                handle,
                pad: 0,
                map_offset: 0,
                vaddr: 0,
                xdna_addr: 0,
            };

            let ret = unsafe {
                ioctl::ioctl(self.fd, ioctl::DRM_IOCTL_AMDXDNA_GET_BO_INFO, &raw mut info)
            };

            match ret {
                Ok(_) => Ok(info),
                Err(e) => Err(DirectError::IoctlFailed(format!(
                    "GET_BO_INFO failed for handle={handle} (request=0x{:x}): {e}",
                    ioctl::DRM_IOCTL_AMDXDNA_GET_BO_INFO
                ))),
            }
        }

        #[cfg(not(feature = "amd-xdna2"))]
        {
            let _ = handle;
            Err(DirectError::Unsupported(
                "get_bo_info requires the amd-xdna2 feature".to_string(),
            ))
        }
    }
}

impl RealDirectSubmission {}

/// Core command types understood by the amdxdna driver (from the uAPI).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum CommandType {
    /// Submit an execution buffer (the main command type for running work).
    SubmitExecBuf = 0,
    /// Declare a dependency between commands.
    SubmitDependency = 1,
    /// Signal completion of a command.
    SubmitSignal = 2,
}

/// Lightweight representation of a hardware context on the NPU.
///
/// This is the Rust-side view of what the driver calls a "hwctx".
#[derive(Debug, Clone)]
pub struct HardwareContext {
    /// Driver-assigned handle for this context.
    pub handle: u32,
    /// Number of AIE tiles allocated to this context.
    pub num_tiles: u32,
    /// Amount of tile memory available.
    pub mem_size: u32,
    /// The UMQ BO handle that was created for this context (for advanced bring-up / cleanup).
    pub umq_bo: u32,
    /// The log buffer BO handle that was created for this context.
    pub log_buf_bo: u32,
}

/// Quality of Service hints for a hardware context (directly from amdxdna uAPI).
///
/// Must be `#[repr(C)]` — `amdxdna_drm_create_hwctx_ioctl` always
/// `copy_from_user`s this struct via `qos_p`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct QosInfo {
    /// Giga operations per second hint.
    pub gops: u32,
    /// Frames per second hint.
    pub fps: u32,
    /// DMA bandwidth hint.
    pub dma_bandwidth: u32,
    /// Latency hint.
    pub latency: u32,
    /// Frame execution time hint.
    pub frame_exec_time: u32,
    /// Priority.
    pub priority: u32,
}

/// Buffer object handle as seen by the driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferHandle(pub u32);

/// Basic command submission request for the direct path.
///
/// This is intentionally minimal and focused. We will evolve it as we
/// learn the exact command format needed for real models.
#[derive(Debug, Clone)]
pub struct Command {
    /// Type of command (exec buf, dependency, signal, etc.).
    pub ty: CommandType,
    /// The hardware context this command targets.
    pub hwctx_handle: u32,
    /// Opaque payload for now. Will become structured command BOs later.
    pub payload: Vec<u8>,

    /// Optional handle to a command buffer object (created with BufferType::Cmd).
    /// When present, this value is placed in the `cmd_handles` field of the
    /// amdxdna_drm_exec_cmd request (and cmd_count set to 1).
    ///
    /// This is the next step in submission scaffolding after being able to
    /// create real Cmd BOs: wiring the handle into EXEC_CMD.
    pub cmd_bo_handle: Option<u32>,
}

// =============================================================================
// EXEC_CMD payload research scaffold (step 3) + timed op API (step 4)
// =============================================================================
//
// Provenance (public sources only, 2026-07):
// - amdxdna_accel.h: EXEC_CMD ioctl, BO types, create_hwctx fields
// - xdna-driver aie2_ctx.c / amdxdna_ctx.c (GPL-2.0, github.com/amd/xdna-driver):
//     * force_cmdlist=true by default → cmdlist single/multi execbuf path
//     * ERT_CMD_CHAIN / ERT_CMD_STATE_* for command BO state words
//     * OP_USER / OP_NOOP / OP_SYNC_BO opcodes in amdxdna_cmd_submit
//     * "Transaction binaries" require RAI 1.5+ firmware (older → -EOPNOTSUPP)
// - Public headers do NOT document the full txn-op / control-code encoding that
//   the firmware executes inside AMDXDNA_BO_CMD buffers. That remains the gap.
//
// Until a redistributable known-good blob or fully reverse-engineered layout is
// available, we expose:
//   1) `ExecCmdPayloadScaffold` — documented header words + zero body
//   2) `timed_matmul` — honest Result that measures host-side scaffolding time
//      and returns structured timings (or clear Unsupported/IoctlFailed)

/// Minimal header words observed / inferred for command BOs (research scaffold).
///
/// Layout is **not** claimed complete. Fields mirror ERT-style state words used
/// by the driver (`amdxdna_cmd_set_state` / `amdxdna_cmd_get_op`) so bring-up
/// code can fill a Cmd BO before EXEC_CMD without inventing proprietary formats.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ExecCmdPayloadHeader {
    /// ERT-style state (e.g. NEW=1 before submit; driver may overwrite).
    pub state: u32,
    /// Opcode / custom opcode slot (0 = unset / research placeholder).
    pub opcode: u32,
    /// Count of following dwords in the payload body (0 for empty scaffold).
    pub count: u32,
    /// Reserved / alignment.
    pub reserved: u32,
}

impl Default for ExecCmdPayloadHeader {
    fn default() -> Self {
        Self {
            // ERT_CMD_STATE_NEW is commonly 1 in XRT/ERT headers; driver sets NEW
            // itself on submit in aie2_sched_job_run — we still seed a sane value.
            state: 1,
            opcode: 0,
            count: 0,
            reserved: 0,
        }
    }
}

/// Whether the process is allowed to issue `DRM_IOCTL_AMDXDNA_EXEC_CMD` from
/// **userspace** (L1 gates only).
///
/// **Permanent safety policy (docs/XDNA2_NEVER_AGAIN.md):**
/// 1. Cargo feature `xdna2-dangerous-exec-cmd` must be enabled at **build** time
///    (not implied by `amd-xdna2` alone).
/// 2. Env `XDNA2_ALLOW_EXEC_CMD=1` (or `true`) must be set at **runtime**.
///
/// Legacy env `XDNA2_TRY_SUBMIT` is **ignored** (retired after 2026-07-13).
/// Default builds can never re-trigger the cmdlist oops via this path.
///
/// **Kernel L3 (if a lab-patched `amdxdna` is installed):** module param
/// `exec_cmd_allow` defaults to 0 and returns `-EPERM` for the entire EXEC_CMD
/// ioctl until `modprobe amdxdna exec_cmd_allow=1`. That is **not** reflected
/// here (userspace cannot probe it without opening the device). See
/// [`exec_cmd_block_reason`] and `kernel/patches/README.md`.
///
/// This function (with [`exec_cmd_block_reason`]) is the **single source of
/// truth** for **userspace** EXEC_CMD permission. Callers must not re-implement
/// ad-hoc gates.
#[must_use]
pub fn exec_cmd_is_enabled() -> bool {
    #[cfg(not(feature = "xdna2-dangerous-exec-cmd"))]
    {
        false
    }
    #[cfg(feature = "xdna2-dangerous-exec-cmd")]
    {
        std::env::var("XDNA2_ALLOW_EXEC_CMD")
            .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
    }
}

/// Human-readable reason EXEC_CMD is blocked (for errors / probe output).
#[must_use]
pub fn exec_cmd_block_reason() -> &'static str {
    #[cfg(not(feature = "xdna2-dangerous-exec-cmd"))]
    {
        "EXEC_CMD disabled at compile time (enable Cargo feature \
         xdna2-dangerous-exec-cmd AND set XDNA2_ALLOW_EXEC_CMD=1; see docs/XDNA2_NEVER_AGAIN.md)"
    }
    #[cfg(feature = "xdna2-dangerous-exec-cmd")]
    {
        if exec_cmd_is_enabled() {
            "EXEC_CMD userspace gates open (feature + XDNA2_ALLOW_EXEC_CMD=1); \
             if a lab L3-patched amdxdna is loaded, also need exec_cmd_allow=1 \
             or kernel returns -EPERM (see docs/XDNA2_NEVER_AGAIN.md)"
        } else {
            "EXEC_CMD blocked: set XDNA2_ALLOW_EXEC_CMD=1 (feature xdna2-dangerous-exec-cmd is on; \
             scaffold payloads can oops amdxdna — see docs/XDNA2_NEVER_AGAIN.md)"
        }
    }
}

/// Build a minimal command-BO byte buffer for EXEC_CMD experiments.
///
/// Returns `header` as little-endian bytes followed by `body_dwords` zeroed
/// u32 slots. Does **not** encode a real firmware txn; use for ioctl plumbing
/// and dmesg reaction only until payload research lands a known-good blob.
///
/// **Does not submit.** Calling this is safe; issuing EXEC_CMD is gated by
/// [`exec_cmd_is_enabled`].
pub fn build_exec_cmd_payload_scaffold(body_dwords: usize) -> Vec<u8> {
    let header = ExecCmdPayloadHeader {
        count: u32::try_from(body_dwords).unwrap_or(u32::MAX),
        ..ExecCmdPayloadHeader::default()
    };
    let mut out = Vec::with_capacity(16 + body_dwords.saturating_mul(4));
    out.extend_from_slice(&header.state.to_le_bytes());
    out.extend_from_slice(&header.opcode.to_le_bytes());
    out.extend_from_slice(&header.count.to_le_bytes());
    out.extend_from_slice(&header.reserved.to_le_bytes());
    out.resize(out.len() + body_dwords.saturating_mul(4), 0);
    out
}

/// Result of a timed buffer / matmul attempt on the direct path.
///
/// Designed so colibrì tools can parse the same fields whether the op ran on
/// silicon or returned an honest skip/error.
#[derive(Debug, Clone)]
pub struct TimedOpResult {
    /// Logical op name (`"matmul"`, `"copy"`, `"noop_probe"`, …).
    pub op: String,
    /// Matrix M dimension (rows of A / output); 0 if N/A.
    pub m: u32,
    /// Matrix N dimension (cols of B / output); 0 if N/A.
    pub n: u32,
    /// Matrix K dimension (inner); 0 if N/A.
    pub k: u32,
    /// Dtype string (`"f32"`, `"bf16"`, …).
    pub dtype: String,
    /// Wall time for the whole attempt (host clock), seconds.
    pub total_s: f64,
    /// Host-side prep time (BO create / payload fill), seconds.
    pub host_prep_s: f64,
    /// Device / submit time if a real EXEC_CMD was issued, else `None`.
    pub device_s: Option<f64>,
    /// Whether real NPU compute completed successfully.
    pub ran_on_npu: bool,
    /// Stable machine status: `ok`, `hwctx_blocked`, `payload_unsupported`, `no_device`, …
    pub status: String,
    /// Human-readable detail (error text or notes).
    pub detail: String,
}

impl TimedOpResult {
    /// Serialize as a single JSON object (serde_json) for CLI / colibrì tools.
    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "op": self.op,
            "m": self.m,
            "n": self.n,
            "k": self.k,
            "dtype": self.dtype,
            "total_s": self.total_s,
            "host_prep_s": self.host_prep_s,
            "device_s": self.device_s,
            "ran_on_npu": self.ran_on_npu,
            "status": self.status,
            "detail": self.detail,
        })
    }
}

/// Attempt a minimal timed matmul (or honest structured failure).
///
/// **Current honesty:** CREATE_HWCTX and/or firmware txn payload still block
/// real GEMM on silicon. This function:
/// 1. Tries `create_hwctx` (with valid qos_p + DevHeap path).
/// 2. On success, issues scaffold Cmd BO + EXEC_CMD **only if**
///    [`exec_cmd_is_enabled`] (feature + `XDNA2_ALLOW_EXEC_CMD=1`); otherwise
///    returns status `hwctx_ok` and never calls `submit_command`.
/// 3. Always returns a `TimedOpResult` with timings and a clear `status` string
///    that colibrì can map to skip taxonomy — never panics, never fakes Pass
///    (`ran_on_npu` stays false for scaffold).
///
/// `m,n,k` are recorded for gate tables even when the op does not run on NPU.
#[allow(clippy::too_many_lines)]
pub fn timed_matmul_direct(
    submission: &dyn DirectSubmission,
    m: u32,
    n: u32,
    k: u32,
    dtype: &str,
) -> TimedOpResult {
    use std::time::Instant;

    let t0 = Instant::now();
    let mut host_prep_s = 0.0_f64;

    // Prefer tile counts that divide evenly by AIE2 core row_count (often 4).
    // aie2_ctx_init: orig_num_col = num_tiles / row_count.
    let tiles = parse_u32_from_env("XDNA2_HWCTX_TILES_SINGLE", 4).max(1);
    let mem = parse_u32_from_env("XDNA2_HWCTX_MEM_SINGLE", 256 * 1024);
    // Prefer fixed sizes for timed path (avoid multi-minute probe matrix).
    let umq = parse_u64_from_env("XDNA2_HWCTX_UMQ", 16 * 1024).max(4096);
    let log = parse_u64_from_env("XDNA2_HWCTX_LOG", 128 * 1024).max(4096);

    let prep0 = Instant::now();
    let qos = QosInfo::default();
    let hwctx = submission.create_hwctx_with_sizes(0, tiles, mem, umq, log, Some(&qos));
    host_prep_s += prep0.elapsed().as_secs_f64();

    let ctx = match hwctx {
        Ok(c) => c,
        Err(e) => {
            let total_s = t0.elapsed().as_secs_f64();
            return TimedOpResult {
                op: "matmul".to_string(),
                m,
                n,
                k,
                dtype: dtype.to_string(),
                total_s,
                host_prep_s,
                device_s: None,
                ran_on_npu: false,
                status: "hwctx_blocked".to_string(),
                detail: format!(
                    "CREATE_HWCTX failed (direct path): {e}. \
                         Real GEMM requires a working hwctx + firmware txn payload."
                ),
            };
        }
    };

    // CRITICAL SAFETY: see exec_cmd_is_enabled() / docs/XDNA2_NEVER_AGAIN.md.
    // Default path never issues EXEC_CMD (compile + runtime double gate).
    let try_submit = exec_cmd_is_enabled();

    let (ran, device_s, status, detail) = if try_submit {
        // Scaffold command BO + EXEC_CMD (not a real matmul payload).
        let prep1 = Instant::now();
        let payload = build_exec_cmd_payload_scaffold(8);
        let cmd_bo = submission.create_buffer(0, payload.len() as u64, BufferType::Cmd);
        host_prep_s += prep1.elapsed().as_secs_f64();

        match cmd_bo {
            Ok(handle) => {
                let cmd = Command {
                    ty: CommandType::SubmitExecBuf,
                    hwctx_handle: ctx.handle,
                    payload,
                    cmd_bo_handle: Some(handle.0),
                };
                let d0 = Instant::now();
                let submit = submission.submit_command(0, &cmd);
                let ds = d0.elapsed().as_secs_f64();
                let _ = submission.destroy_buffer(0, handle.0);
                match submit {
                    Ok(seq) => (
                        false, // never claim NPU compute Pass from scaffold
                        Some(ds),
                        "ok_submit_scaffold".to_string(),
                        format!(
                            "EXEC_CMD accepted seq={seq}; payload is research scaffold only \
                             (not a verified GEMM). Destroying hwctx."
                        ),
                    ),
                    Err(e) => (
                        false,
                        Some(ds),
                        "payload_unsupported".to_string(),
                        format!(
                            "hwctx ok (handle={}) but EXEC_CMD failed: {e}. \
                             Firmware txn format still the research gap.",
                            ctx.handle
                        ),
                    ),
                }
            }
            Err(e) => (
                false,
                None,
                "cmd_bo_failed".to_string(),
                format!("CREATE_BO(Cmd) failed after hwctx success: {e}"),
            ),
        }
    } else {
        (
            false,
            None,
            "hwctx_ok".to_string(),
            format!(
                "CREATE_HWCTX succeeded (handle={}); EXEC_CMD not attempted ({}). \
                 Destroying hwctx.",
                ctx.handle,
                exec_cmd_block_reason()
            ),
        )
    };

    let _ = submission.destroy_hwctx(0, ctx.handle);

    TimedOpResult {
        op: "matmul".to_string(),
        m,
        n,
        k,
        dtype: dtype.to_string(),
        total_s: t0.elapsed().as_secs_f64(),
        host_prep_s,
        device_s,
        ran_on_npu: ran,
        status,
        detail,
    }
}

/// Trait-like interface for the lightweight direct execution path.
///
/// We define this as a trait so we can have different concrete
/// implementations over time (pure ioctl, a thin wrapper, testing fakes, etc.)
/// without polluting the rest of the real execution code.
pub trait DirectSubmission {
    /// Create a hardware context on the device with optional QoS hints.
    ///
    /// This is the direct equivalent of DRM_AMDXDNA_CREATE_HWCTX.
    fn create_hwctx(
        &self,
        fd: RawFd,
        num_tiles: u32,
        mem_size: u32,
        qos: Option<&QosInfo>,
    ) -> Result<HardwareContext, DirectError>;

    /// Create a hardware context with explicit UMQ and log buffer sizes.
    ///
    /// Prefer this over the multi-size probe matrix when you already know good
    /// sizes (or want a fast single-shot attempt). Still tries KMQ zero-BO first.
    ///
    /// Default implementation ignores sizes and calls [`Self::create_hwctx`].
    fn create_hwctx_with_sizes(
        &self,
        fd: RawFd,
        num_tiles: u32,
        mem_size: u32,
        umq_size: u64,
        log_size: u64,
        qos: Option<&QosInfo>,
    ) -> Result<HardwareContext, DirectError> {
        let _ = (umq_size, log_size);
        self.create_hwctx(fd, num_tiles, mem_size, qos)
    }

    /// Create a hardware context, forcing a single `bo_flags` value for the
    /// UMQ and log buffer CREATE_BO calls (instead of the internal multi-value
    /// flags matrix). This is intended for targeted bring-up experiments
    /// (e.g. driven by XDNA2_BO_FLAGS env in the probe/example) once the user
    /// has seen what the default rich matrix produces on their hardware.
    ///
    /// Default implementation falls back to the normal create_hwctx (for Stubs
    /// and older impls).
    fn create_hwctx_with_forced_bo_flags(
        &self,
        fd: RawFd,
        num_tiles: u32,
        mem_size: u32,
        qos: Option<&QosInfo>,
        _bo_flags: u64,
    ) -> Result<HardwareContext, DirectError> {
        self.create_hwctx(fd, num_tiles, mem_size, qos)
    }

    /// Destroy a previously created hardware context.
    fn destroy_hwctx(&self, fd: RawFd, handle: u32) -> Result<(), DirectError>;

    /// Destroy a buffer object.
    fn destroy_buffer(&self, _fd: RawFd, _handle: u32) -> Result<(), DirectError> {
        Err(DirectError::Unsupported(
            "destroy_buffer not implemented for this submission backend".to_string(),
        ))
    }

    /// Create a buffer object visible to the NPU.
    ///
    /// This is the direct equivalent of DRM_AMDXDNA_CREATE_BO.
    fn create_buffer(
        &self,
        fd: RawFd,
        size: u64,
        ty: BufferType,
    ) -> Result<BufferHandle, DirectError>;

    /// Submit a command for execution on a hardware context.
    ///
    /// This is the direct equivalent of DRM_AMDXDNA_EXEC_CMD.
    fn submit_command(&self, fd: RawFd, cmd: &Command) -> Result<u64, DirectError>; // returns seq

    /// Sync a buffer (direction to/from device).
    ///
    /// Direct equivalent of DRM_AMDXDNA_SYNC_BO.
    fn sync_buffer(
        &self,
        fd: RawFd,
        handle: BufferHandle,
        direction: u32,
        offset: u64,
        size: u64,
    ) -> Result<(), DirectError>;

    /// Query information from the device (power mode, firmware version, etc.).
    ///
    /// Direct equivalent of DRM_AMDXDNA_GET_INFO for various params.
    fn get_info(&self, fd: RawFd, param: u32, buffer: &mut [u8]) -> Result<(), DirectError>;

    /// Convenience: get firmware version (high-level typed query).
    /// Default implementation returns unsupported; real impls override it.
    fn get_firmware_version(&self) -> Result<amdxdna_drm_query_firmware_version, DirectError> {
        Err(DirectError::Unsupported(
            "get_firmware_version not implemented for this submission backend".to_string(),
        ))
    }

    /// Convenience: get configured power mode.
    fn get_power_mode(&self) -> Result<amdxdna_drm_get_power_mode, DirectError> {
        Err(DirectError::Unsupported(
            "get_power_mode not implemented for this submission backend".to_string(),
        ))
    }

    /// Convenience: get resource information (TOPS, clock, task limits, etc.).
    fn get_resource_info(&self) -> Result<amdxdna_drm_get_resource_info, DirectError> {
        Err(DirectError::Unsupported(
            "get_resource_info not implemented for this submission backend".to_string(),
        ))
    }

    /// Convenience: get a sensor reading (temp, power, etc.).
    fn get_sensors(&self) -> Result<amdxdna_drm_query_sensor, DirectError> {
        Err(DirectError::Unsupported(
            "get_sensors not implemented for this submission backend".to_string(),
        ))
    }

    /// Query AIE version from the device.
    fn get_aie_version(&self) -> Result<amdxdna_drm_query_aie_version, DirectError> {
        Err(DirectError::Unsupported(
            "get_aie_version not implemented for this submission backend".to_string(),
        ))
    }

    /// Get information about a buffer object (map offset, device address, etc.).
    fn get_bo_info(
        &self,
        _fd: RawFd,
        _handle: u32,
    ) -> Result<amdxdna_drm_get_bo_info, DirectError> {
        Err(DirectError::Unsupported(
            "get_bo_info not implemented for this submission backend".to_string(),
        ))
    }
}

/// Supported buffer object types (mirrors the kernel uAPI).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BufferType {
    /// Regular shared memory buffer.
    Shmem = 1,
    /// Device heap buffer.
    DevHeap = 2,
    /// Device-only buffer.
    Dev = 3,
    /// Buffer used to hold commands.
    Cmd = 4,
}

/// Errors specific to the lightweight direct driver path.
#[derive(Debug, thiserror::Error)]
pub enum DirectError {
    /// An ioctl call to the amdxdna driver failed.
    #[error("ioctl failed: {0}")]
    IoctlFailed(String),

    /// The device or driver does not support a required feature.
    #[error("device does not support required feature: {0}")]
    Unsupported(String),

    /// An invalid argument was passed to a direct driver operation.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    /// The amdxdna driver returned a specific error code.
    #[error("driver returned error: {0}")]
    DriverError(i32),
}

/// Placeholder implementation of the lightweight direct submission path.
///
/// Used while the real ioctl-based command submission is being built.
/// Clearly signals that no actual hardware execution is happening yet.
///
/// In real use, this (or a real impl) will be created with the open fd
/// from RealXdna2Device so that all operations use the correct device.
#[derive(Debug)]
pub struct StubDirectSubmission {
    /// The raw fd we will eventually use for ioctls.
    /// Currently unused in the stub, but captured for API compatibility
    /// and to make the transition to real code trivial.
    _fd: RawFd,
}

impl Default for StubDirectSubmission {
    fn default() -> Self {
        Self { _fd: -1 }
    }
}

impl StubDirectSubmission {
    /// Create a stub submission context for the given open fd.
    /// The fd is captured even though the stub does nothing with it yet.
    pub fn new(fd: RawFd) -> Self {
        Self { _fd: fd }
    }
}

impl DirectSubmission for StubDirectSubmission {
    fn create_hwctx(
        &self,
        _fd: RawFd, // ignored in stub; real impl will use self._fd or passed fd
        num_tiles: u32,
        mem_size: u32,
        _qos: Option<&QosInfo>,
    ) -> Result<HardwareContext, DirectError> {
        // In a real implementation we would issue DRM_IOCTL_AMDXDNA_CREATE_HWCTX here,
        // passing the QoS struct when provided, using the captured fd.
        Ok(HardwareContext {
            handle: 1,
            num_tiles,
            mem_size,
            umq_bo: 0,
            log_buf_bo: 0,
        })
    }

    fn destroy_hwctx(&self, _fd: RawFd, _handle: u32) -> Result<(), DirectError> {
        Ok(())
    }

    fn create_buffer(
        &self,
        _fd: RawFd,
        size: u64,
        ty: BufferType,
    ) -> Result<BufferHandle, DirectError> {
        // Would issue DRM_IOCTL_AMDXDNA_CREATE_BO
        let _ = (size, ty);
        Ok(BufferHandle(42))
    }

    fn submit_command(&self, _fd: RawFd, cmd: &Command) -> Result<u64, DirectError> {
        // Same permission surface as Real: single source of truth for EXEC_CMD.
        // Stub never issues an ioctl either way; messaging stays consistent when gated.
        if !exec_cmd_is_enabled() {
            return Err(DirectError::Unsupported(
                exec_cmd_block_reason().to_string(),
            ));
        }
        // Would issue DRM_IOCTL_AMDXDNA_EXEC_CMD using the structures from amdxdna_accel.h
        Err(DirectError::Unsupported(format!(
            "stub: EXEC_CMD allowed but real payload/submission not implemented (cmd type: {:?})",
            cmd.ty
        )))
    }

    fn sync_buffer(
        &self,
        _fd: RawFd,
        _handle: BufferHandle,
        _direction: u32,
        _offset: u64,
        _size: u64,
    ) -> Result<(), DirectError> {
        Ok(())
    }

    fn get_info(&self, _fd: RawFd, _param: u32, _buffer: &mut [u8]) -> Result<(), DirectError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_submission_returns_clear_error_on_submit() {
        let stub = StubDirectSubmission { _fd: -1 };
        let cmd = Command {
            ty: CommandType::SubmitExecBuf,
            hwctx_handle: 1,
            payload: vec![],
            cmd_bo_handle: None,
        };

        let result = stub.submit_command(0, &cmd);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, DirectError::Unsupported(_)));
    }

    #[test]
    fn buffer_types_have_correct_values() {
        assert_eq!(BufferType::Shmem as u32, 1);
        assert_eq!(BufferType::Cmd as u32, 4);
    }

    #[test]
    fn qos_info_defaults_are_zero() {
        let qos = QosInfo::default();
        assert_eq!(qos.gops, 0);
        assert_eq!(qos.priority, 0);
    }

    #[test]
    fn stub_create_hwctx_succeeds() {
        let stub = StubDirectSubmission { _fd: -1 };
        let qos = QosInfo {
            gops: 100,
            priority: 1,
            ..Default::default()
        };

        let ctx = stub
            .create_hwctx(0, 4, 1024 * 1024, Some(&qos))
            .expect("stub create_hwctx should succeed");

        assert_eq!(ctx.num_tiles, 4);
        assert_eq!(ctx.mem_size, 1024 * 1024);
    }

    #[test]
    fn stub_sync_and_get_info_are_noops() {
        let stub = StubDirectSubmission { _fd: -1 };
        assert!(stub.sync_buffer(0, BufferHandle(1), 0, 0, 4096).is_ok());
        assert!(stub.get_info(0, 0, &mut [0u8; 64]).is_ok());
    }

    #[test]
    fn ported_bo_structs_have_reasonable_layout() {
        // Basic sanity that our ports compile and have the fields we expect.
        // When we do real ioctls these must match the kernel exactly.
        let create = amdxdna_drm_create_bo {
            flags: 0,
            vaddr: 0,
            size: 4096,
            ty: AmdXdnaBoType::Shmem as u32,
            handle: 0,
        };
        assert_eq!(create.size, 4096);

        let info = amdxdna_drm_get_bo_info {
            ext: 0,
            ext_flags: 0,
            handle: 42,
            pad: 0,
            map_offset: 0,
            vaddr: 0,
            xdna_addr: 0,
        };
        assert_eq!(info.handle, 42);

        let sync = amdxdna_drm_sync_bo {
            handle: 1,
            direction: 0,
            offset: 0,
            size: 4096,
        };
        assert_eq!(sync.size, 4096);
    }

    #[test]
    fn real_direct_submission_can_be_constructed_with_fd() {
        // This is the shape the real implementation will take.
        let _real = RealDirectSubmission::new(42);
    }

    #[test]
    fn ported_query_structs_compile() {
        let fw = amdxdna_drm_query_firmware_version {
            major: 0,
            minor: 0,
            patch: 0,
            build: 0,
        };
        assert_eq!(fw.major, 0);
        let pwr = amdxdna_drm_get_power_mode {
            power_mode: 0,
            pad: [0; 7],
        };
        assert_eq!(pwr.power_mode, 0);
        let res = amdxdna_drm_get_resource_info {
            npu_clk_max: 0,
            npu_tops_max: 0,
            npu_task_max: 0,
            npu_tops_curr: 0,
            npu_task_curr: 0,
        };
        assert_eq!(res.npu_tops_max, 0);
    }

    #[test]
    fn real_direct_submission_has_firmware_and_power_helpers() {
        let real = RealDirectSubmission::new(42);
        // These should be callable (they will attempt real ioctls on hardware).
        let _ = real.get_firmware_version();
        let _ = real.get_power_mode();
        let _ = real.get_resource_info();
    }

    #[test]
    fn stub_submission_returns_unsupported_for_new_typed_queries() {
        let stub = StubDirectSubmission { _fd: -1 };
        assert!(matches!(
            stub.get_firmware_version(),
            Err(DirectError::Unsupported(_))
        ));
        assert!(matches!(
            stub.get_power_mode(),
            Err(DirectError::Unsupported(_))
        ));
        assert!(matches!(
            stub.get_resource_info(),
            Err(DirectError::Unsupported(_))
        ));
    }

    #[test]
    fn amdxdna_drm_exec_cmd_struct_size_is_exactly_56_bytes() {
        // Critical regression guard for the direct submission path.
        //
        // The DRM_IOCTL_AMDXDNA_EXEC_CMD request code (0xc038_6446) is derived
        // from sizeof(this struct). If the ported struct definition ever drifts
        // from the kernel's amdxdna_drm_exec_cmd (extra fields, different packing,
        // new members in a header sync), this test will fail and force us to
        // recompute + re-validate the ioctl constant before any real EXEC_CMD
        // experiments on hardware.
        //
        // Current layout (must stay 56 bytes for the current 0xc038_6446 to be correct):
        //   ext u64 + ext_flags u64 + hwctx u32 + ty u32 + cmd_handles u64 +
        //   args u64 + cmd_count u32 + arg_count u32 + seq u64 = 56 bytes.
        assert_eq!(
            core::mem::size_of::<amdxdna_drm_exec_cmd>(),
            56,
            "amdxdna_drm_exec_cmd size changed; update DRM_IOCTL_AMDXDNA_EXEC_CMD constant derivation and all submit_command scaffolding"
        );
    }

    #[test]
    fn command_with_cmd_bo_handle_constructs_and_submits() {
        // Exercises the new cmd_bo_handle field in Command and the submit path.
        let cmd = Command {
            ty: CommandType::SubmitExecBuf,
            hwctx_handle: 42,
            payload: vec![],
            cmd_bo_handle: Some(123),
        };

        assert_eq!(cmd.cmd_bo_handle, Some(123));

        let stub = StubDirectSubmission { _fd: -1 };
        let result = stub.submit_command(0, &cmd);
        // Stub returns Unsupported; the important thing is it compiled, ran,
        // and the field was present (real path now wires it into the ioctl req).
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, DirectError::Unsupported(_)));
    }

    #[test]
    fn parse_bo_flags_list_uses_default_for_empty_or_invalid() {
        assert_eq!(parse_bo_flags_list(""), vec![0, 1, 2, 4]);
        assert_eq!(parse_bo_flags_list("   , ,"), vec![0, 1, 2, 4]);
        assert_eq!(parse_bo_flags_list("bad,0xZZZ"), vec![0, 1, 2, 4]);
    }

    #[test]
    fn parse_bo_flags_list_parses_decimal_and_hex_mixed() {
        assert_eq!(
            parse_bo_flags_list("0,0x10,32,0xFF"),
            vec![0, 0x10, 32, 0xFF]
        );
        assert_eq!(parse_bo_flags_list("1,2,4"), vec![1, 2, 4]);
    }

    #[test]
    fn parse_bo_flags_list_ignores_malformed_entries() {
        assert_eq!(parse_bo_flags_list("0x10,,bad,0x20,   "), vec![0x10, 0x20]);
        assert_eq!(
            parse_bo_flags_list("0x10, 0x20 , 0x30"),
            vec![0x10, 0x20, 0x30]
        );
    }

    #[test]
    fn parse_u32_from_env_returns_default_for_missing_key() {
        // Safe test — only exercises the default path (env mutation is unsafe in modern Rust test contexts).
        assert_eq!(
            parse_u32_from_env("XDNA2_TEST_U32_THIS_KEY_DOES_NOT_EXIST_12345", 123),
            123
        );
    }

    #[test]
    fn parse_u64_from_env_returns_default_for_missing_key() {
        // Safe test — only exercises the default path.
        assert_eq!(
            parse_u64_from_env(
                "XDNA2_TEST_U64_THIS_KEY_DOES_NOT_EXIST_67890",
                0x1234_5678_90AB_CDEF
            ),
            0x1234_5678_90AB_CDEF
        );
    }

    #[test]
    fn build_compact_hwctx_failure_summary_produces_short_useful_output() {
        let logs = vec![
            "UMQ_CREATE_BO_FAILED size=4096 ty=DevHeap flags=0x0: Device or resource busy (os error 16)".to_string(),
            "CREATE_HWCTX_FAILED (tiles=1, mem=262144, umq_bo=1, ...): Invalid argument (os error 22). pwr=0 aie=1.1 res=tops_max=58".to_string(),
            "CREATE_HWCTX_FAILED (tiles=1, mem=1048576, ...): Invalid argument (os error 22)".to_string(),
            "CREATE_HWCTX_FAILED (tiles=2, ...): Invalid argument (os error 22)".to_string(),
        ];

        let summary = build_compact_hwctx_failure_summary(&logs);

        // Must be dramatically shorter than the raw joined string.
        assert!(
            summary.len() < 600,
            "summary was too long: {}",
            summary.len()
        );
        assert!(summary.contains("1 CREATE_BO failures, 3 CREATE_HWCTX failures"));
        assert!(summary.contains("DevHeap EBUSY seen"));
        assert!(summary.contains("4 total"));
        assert!(summary.contains("Sample:"));
    }

    #[test]
    fn build_compact_hwctx_failure_summary_handles_empty_and_small_inputs() {
        assert_eq!(build_compact_hwctx_failure_summary(&[]), "no attempts");

        let tiny = vec!["CREATE_HWCTX_FAILED foo bar".to_string()];
        let s = build_compact_hwctx_failure_summary(&tiny);
        assert!(s.contains("0 CREATE_BO failures, 1 CREATE_HWCTX failures"));
        assert!(s.contains("1 total"));
    }

    #[test]
    fn aie2_dev_heap_size_is_64_mib() {
        // Guard: aie2_ctx_init requires a client DevHeap that is a multiple of
        // AIE2_DEVM_SIZE (64 MiB). If this constant drifts, CREATE_HWCTX will
        // fail with ENOENT / EINVAL on real silicon.
        assert_eq!(AIE2_DEV_HEAP_SIZE, 64 * 1024 * 1024);
    }

    #[test]
    fn create_hwctx_and_get_bo_info_struct_sizes_match_ioctl_constants() {
        // CREATE_HWCTX: size 56 → 0xc038_6440
        assert_eq!(core::mem::size_of::<amdxdna_drm_create_hwctx>(), 56);
        // GET_BO_INFO: size 48 → 0xc030_6444
        assert_eq!(core::mem::size_of::<amdxdna_drm_get_bo_info>(), 48);
        // GEM_CLOSE: size 8 → 0x4008_6409
        assert_eq!(core::mem::size_of::<drm_gem_close>(), 8);
        // CREATE_BO: size 32 → 0xc020_6443
        assert_eq!(core::mem::size_of::<amdxdna_drm_create_bo>(), 32);
    }

    #[test]
    fn buffer_type_dev_heap_is_2() {
        // Must match AMDXDNA_BO_DEV_HEAP in amdxdna_accel.h
        assert_eq!(BufferType::DevHeap as u32, 2);
        assert_eq!(AmdXdnaBoType::DevHeap as u32, 2);
    }

    #[test]
    fn qos_info_is_repr_c_and_24_bytes() {
        // amdxdna_qos_info is 6 * u32 = 24 bytes; CREATE_HWCTX always copy_from_user's it.
        assert_eq!(core::mem::size_of::<QosInfo>(), 24);
    }

    #[test]
    fn create_hwctx_with_sizes_on_stub_succeeds_and_ignores_sizes() {
        let stub = StubDirectSubmission::new(-1);
        let ctx = stub
            .create_hwctx_with_sizes(0, 4, 256 * 1024, 16 * 1024, 128 * 1024, None)
            .expect("stub sized create should succeed");
        assert_eq!(ctx.num_tiles, 4);
        assert_eq!(ctx.mem_size, 256 * 1024);
    }

    #[test]
    fn with_hwctx_size_override_restores_previous() {
        HWCTX_SIZE_OVERRIDE.with(|c| c.set(None));
        with_hwctx_size_override(4096, 8192, || {
            assert_eq!(
                HWCTX_SIZE_OVERRIDE.with(std::cell::Cell::get),
                Some((4096, 8192))
            );
        });
        assert_eq!(HWCTX_SIZE_OVERRIDE.with(std::cell::Cell::get), None);
    }

    #[test]
    fn build_exec_cmd_payload_scaffold_has_header_and_body() {
        let p = build_exec_cmd_payload_scaffold(4);
        assert_eq!(p.len(), 16 + 16); // header 16 + 4 dwords
        // state = 1 (NEW) little-endian
        assert_eq!(&p[0..4], &1u32.to_le_bytes());
        // count = 4
        assert_eq!(&p[8..12], &4u32.to_le_bytes());
    }

    #[test]
    fn exec_cmd_is_disabled_by_default() {
        // Permanent policy: default builds must never enable EXEC_CMD.
        // `amd-xdna2` alone must NOT imply `xdna2-dangerous-exec-cmd`.
        //
        // Lib tests assume the double gate is **closed** in the process:
        // - feature `xdna2-dangerous-exec-cmd` off, **or**
        // - feature on but `XDNA2_ALLOW_EXEC_CMD` unset/not-true.
        // Do not run unit tests with both layers open (see NEVER_AGAIN).
        assert!(
            !exec_cmd_is_enabled(),
            "lib tests require EXEC_CMD gated closed (unset XDNA2_ALLOW_EXEC_CMD; \
             do not pair xdna2-dangerous-exec-cmd with ALLOW=1 for unit tests). \
             See docs/XDNA2_NEVER_AGAIN.md"
        );
        let reason = exec_cmd_block_reason();
        assert!(reason.contains("EXEC_CMD"), "reason={reason}");
        // Without the dangerous feature, reason must mention compile-time gate.
        #[cfg(not(feature = "xdna2-dangerous-exec-cmd"))]
        {
            assert!(
                reason.contains("compile time") || reason.contains("xdna2-dangerous-exec-cmd"),
                "reason={reason}"
            );
        }
        // With feature compiled in, runtime still requires ALLOW env (matrix job).
        #[cfg(feature = "xdna2-dangerous-exec-cmd")]
        {
            assert!(
                reason.contains("XDNA2_ALLOW_EXEC_CMD"),
                "with dangerous feature compiled, block reason must cite env: {reason}"
            );
        }
    }

    #[test]
    fn exec_cmd_source_never_reads_legacy_try_submit() {
        // Regression: reintroducing env::var / var_os of the retired name as an
        // enable path would silently revive the pre-NEVER_AGAIN opt-in.
        // Scan production source only (before the test module) so empirical tests
        // may still set/clear the retired key without false positives.
        // Needle via concat! so this assert's text does not self-match.
        let src = include_str!("direct.rs");
        let prod = src
            .split("#[cfg(test)]")
            .next()
            .expect("direct.rs should contain a #[cfg(test)] module");
        let needle_var = concat!("env::var(", "\"", "XDNA2_TRY_SUBMIT", "\"", ")");
        let needle_var_os = concat!("env::var_os(", "\"", "XDNA2_TRY_SUBMIT", "\"", ")");
        assert!(
            !prod.contains(needle_var) && !prod.contains(needle_var_os),
            "production direct.rs must not read XDNA2_TRY_SUBMIT (retired; use XDNA2_ALLOW_EXEC_CMD + feature)"
        );
    }

    #[test]
    #[allow(unsafe_code)]
    fn exec_cmd_ignores_legacy_xdna2_try_submit_env() {
        // Empirical: setting the retired env must never enable EXEC_CMD by itself.
        // (Feature-off: always false. Feature-on: still needs XDNA2_ALLOW_EXEC_CMD.)
        let prev = std::env::var_os("XDNA2_TRY_SUBMIT");
        // SAFETY: test-only mutation of a key production code must not read.
        // Restored in all exit paths below. No parallel test should depend on this key.
        unsafe {
            std::env::set_var("XDNA2_TRY_SUBMIT", "1");
        }
        let enabled = exec_cmd_is_enabled();
        if let Some(v) = prev {
            unsafe {
                std::env::set_var("XDNA2_TRY_SUBMIT", v);
            }
        } else {
            unsafe {
                std::env::remove_var("XDNA2_TRY_SUBMIT");
            }
        }
        assert!(
            !enabled,
            "XDNA2_TRY_SUBMIT=1 must never enable EXEC_CMD (see docs/XDNA2_NEVER_AGAIN.md)"
        );
    }

    #[test]
    fn real_submit_command_refuses_when_exec_cmd_gated() {
        // Fail loud if the process has the double gate fully open — this test is
        // meaningless (and would hit ioctl on a fake fd) when EXEC_CMD is allowed.
        assert!(
            !exec_cmd_is_enabled(),
            "real_submit_command_refuses_when_exec_cmd_gated requires gated process \
             (unset XDNA2_ALLOW_EXEC_CMD; lib tests must not run with dangerous feature+ALLOW=1)"
        );
        // No /dev/accel: fake fd. Gate must return before any EXEC_CMD ioctl.
        let real = RealDirectSubmission::new(42);
        let cmd = Command {
            ty: CommandType::SubmitExecBuf,
            hwctx_handle: 1,
            payload: vec![0u8; 16],
            cmd_bo_handle: Some(99),
        };
        let result = real.submit_command(0, &cmd);
        assert!(result.is_err(), "gated Real path must not submit");
        match result.unwrap_err() {
            DirectError::Unsupported(msg) => {
                assert!(
                    msg.contains("EXEC_CMD") || msg.contains("xdna2-dangerous-exec-cmd"),
                    "expected clear gate message, got: {msg}"
                );
                // Message must match the single source of truth.
                assert_eq!(msg, exec_cmd_block_reason());
            }
            other => panic!("expected Unsupported gate error, got {other:?}"),
        }
    }

    #[test]
    fn stub_submit_command_uses_exec_cmd_gate_message() {
        assert!(
            !exec_cmd_is_enabled(),
            "stub gate message test requires EXEC_CMD gated closed"
        );
        let stub = StubDirectSubmission::new(-1);
        let cmd = Command {
            ty: CommandType::SubmitExecBuf,
            hwctx_handle: 1,
            payload: vec![],
            cmd_bo_handle: None,
        };
        match stub.submit_command(0, &cmd).unwrap_err() {
            DirectError::Unsupported(msg) => assert_eq!(msg, exec_cmd_block_reason()),
            other => panic!("expected Unsupported, got {other:?}"),
        }
    }

    #[test]
    fn timed_matmul_direct_on_stub_returns_structured_result() {
        let stub = StubDirectSubmission::new(-1);
        let r = timed_matmul_direct(&stub, 1, 256, 1024, "f32");
        assert_eq!(r.op, "matmul");
        assert_eq!(r.m, 1);
        assert_eq!(r.n, 256);
        assert_eq!(r.k, 1024);
        assert_eq!(r.dtype, "f32");
        assert!(r.total_s >= 0.0);
        assert!(!r.ran_on_npu);
        // Branch on gate so a deliberate dangerous+ALLOW process still has honest asserts
        // (lib tests normally run with gate closed → hwctx_ok).
        if exec_cmd_is_enabled() {
            // Stub submit is attempted; never claim NPU Pass.
            assert!(
                r.status == "payload_unsupported"
                    || r.status == "ok_submit_scaffold"
                    || r.status == "cmd_bo_failed",
                "unexpected status when EXEC_CMD enabled: {}",
                r.status
            );
        } else {
            assert_eq!(r.status, "hwctx_ok");
            assert!(
                r.detail.contains("EXEC_CMD not attempted")
                    || r.detail.contains(exec_cmd_block_reason()),
                "detail should explain gate: {}",
                r.detail
            );
        }
        assert!(!r.status.is_empty());
        let v = r.to_json_value();
        assert_eq!(v["op"], "matmul");
        assert_eq!(v["m"], 1);
        assert_eq!(v["ran_on_npu"], false);
    }

    #[test]
    fn direct_capability_checklist_names_are_stable() {
        let names = DirectCapability::all_ids();
        assert!(names.contains(&"open"));
        assert!(names.contains(&"get_info"));
        assert!(names.contains(&"create_bo"));
        assert!(names.contains(&"sync_bo"));
        assert!(names.contains(&"create_hwctx"));
        assert!(names.contains(&"destroy_hwctx"));
        assert!(names.contains(&"submit"));
        assert!(names.contains(&"timed_gemm"));
    }
}

/// Frozen Direct NPU capability checklist (plan step 1 / colibrì contract).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectCapability {
    /// Open `/dev/accel/accelN`.
    Open,
    /// Typed GET_INFO queries (firmware, power, resources, …).
    GetInfo,
    /// CREATE_BO.
    CreateBo,
    /// SYNC_BO.
    SyncBo,
    /// CREATE_HWCTX (+ DevHeap + qos_p requirements).
    CreateHwctx,
    /// DESTROY_HWCTX / GEM_CLOSE cleanup.
    DestroyHwctx,
    /// EXEC_CMD ioctl path (permanently double-gated; payload may still be scaffold).
    Submit,
    /// Timed matmul / buffer op with structured timings.
    TimedGemm,
}

impl DirectCapability {
    /// Stable string ids for docs / JSON.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::GetInfo => "get_info",
            Self::CreateBo => "create_bo",
            Self::SyncBo => "sync_bo",
            Self::CreateHwctx => "create_hwctx",
            Self::DestroyHwctx => "destroy_hwctx",
            Self::Submit => "submit",
            Self::TimedGemm => "timed_gemm",
        }
    }

    /// All checklist ids in freeze order.
    pub fn all_ids() -> &'static [&'static str] {
        &[
            "open",
            "get_info",
            "create_bo",
            "sync_bo",
            "create_hwctx",
            "destroy_hwctx",
            "submit",
            "timed_gemm",
        ]
    }
}
