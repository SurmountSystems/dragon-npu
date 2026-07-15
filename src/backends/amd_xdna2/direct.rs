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

/// Direct port of `struct amdxdna_drm_wait_cmd` (wait for EXEC_CMD `seq`).
///
/// Issued by [`DirectSubmission::wait_command`] via `DRM_IOCTL_AMDXDNA_WAIT_CMD`
/// (`DRM_IOWR`, request `0xc010_6449` for this 16-byte struct).
///
/// **Gating:** WAIT_CMD is **not** gated by [`exec_cmd_is_enabled`] — waiting
/// without a prior submit is harmless/useless. **EXEC_CMD submit remains gated.**
/// Lab helpers such as [`submit_and_wait_command`] refuse infinite wait
/// (`timeout == 0`); prefer a finite timeout always.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct amdxdna_drm_wait_cmd {
    pub hwctx: u32,
    /// Timeout in milliseconds; 0 means infinite wait (driver semantics).
    pub timeout: u32,
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

/// Direct port of `struct amdxdna_drm_set_power_mode` (SET_STATE buffer).
///
/// Same layout as get (u8 mode + 7 pad). Written via [`DRM_IOCTL_AMDXDNA_SET_STATE`]
/// with param `DRM_AMDXDNA_SET_POWER_MODE` (= 0).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct amdxdna_drm_set_power_mode {
    pub power_mode: u8,
    pub pad: [u8; 7],
}

/// Direct port of `struct amdxdna_drm_set_state` (DRM_AMDXDNA_SET_STATE).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct amdxdna_drm_set_state {
    pub param: u32,
    pub buffer_size: u32,
    pub buffer: u64,
}

/// `enum amdxdna_power_mode_type` (header values).
pub mod power_mode {
    /// Fallback to calculated DPM.
    pub const DEFAULT: u8 = 0;
    /// Lowest DPM frequency.
    pub const LOW: u8 = 1;
    /// Medium DPM.
    pub const MEDIUM: u8 = 2;
    /// Highest normal DPM.
    pub const HIGH: u8 = 3;
    /// Maximum power (turbo).
    pub const TURBO: u8 = 4;
}

/// `enum amdxdna_drm_set_param`: `DRM_AMDXDNA_SET_POWER_MODE` = 0.
pub const AMDXDNA_SET_PARAM_POWER_MODE: u32 = 0;

/// Default hwctx tile count when resource info is unavailable (AIE2-friendly ×4).
pub const DEFAULT_HWCTX_TILES: u32 = 4;
/// Upper clamp for suggested tiles (bring-up default; not a silicon TOPS claim).
pub const MAX_SUGGESTED_HWCTX_TILES: u32 = 8;

/// Suggest CREATE_HWCTX `num_tiles` from resource info.
///
/// Heuristic only (Phase 6 observability/config):
/// - Always a multiple of 4 (AIE2 core-row friendliness used elsewhere in timed paths).
/// - Clamped to [`DEFAULT_HWCTX_TILES`]..=[`MAX_SUGGESTED_HWCTX_TILES`] (4 or 8).
/// - Higher `npu_tops_max` (≥40, e.g. Strix 50+) → 8; else 4.
///
/// Does **not** claim optimal silicon utilization or GEMM efficiency.
#[must_use]
pub fn suggest_hwctx_tiles(resource: &amdxdna_drm_get_resource_info) -> u32 {
    suggest_hwctx_tiles_from_tops(resource.npu_tops_max)
}

/// Same as [`suggest_hwctx_tiles`] from a raw TOPS max (unit tests / offline).
#[must_use]
pub fn suggest_hwctx_tiles_from_tops(tops_max: u64) -> u32 {
    let raw = if tops_max >= 40 {
        MAX_SUGGESTED_HWCTX_TILES
    } else {
        DEFAULT_HWCTX_TILES
    };
    // Snap down to multiple of 4 within [4, 8].
    let snapped = (raw / 4).max(1) * 4;
    snapped.clamp(DEFAULT_HWCTX_TILES, MAX_SUGGESTED_HWCTX_TILES)
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

    // SET_STATE (nr=8): DRM_IOWR('d', 0x40+8, sizeof(amdxdna_drm_set_state)=16)
    // Same size/dir as GET_INFO (nr=7 → 0xc010_6447) → 0xc010_6448.
    // Used for SET_POWER_MODE and other set_param values. Not EXEC_CMD.
    #[allow(dead_code)]
    pub const DRM_IOCTL_AMDXDNA_SET_STATE: libc::c_ulong = 0xc010_6448;

    // WAIT_CMD (nr=9): DRM_IOWR('d', 0x40+9, sizeof(amdxdna_drm_wait_cmd)=16)
    //
    // Ground truth for the **driver** (what amdxdna is compiled against):
    //   src/include/uapi/drm_local/amdxdna_accel.h
    //     #define DRM_IOCTL_AMDXDNA_WAIT_CMD \
    //         DRM_IOWR(DRM_COMMAND_BASE + DRM_AMDXDNA_WAIT_CMD, struct amdxdna_drm_wait_cmd)
    //
    // Derivation (same helper as GET_INFO / EXEC_CMD):
    //   - DRM_AMDXDNA_WAIT_CMD = 9 → nr = 0x40+9 = 0x49
    //   - DRM_IOCTL_BASE = 'd'
    //   - dir = _IOC_READ | _IOC_WRITE = 3
    //   - size = 0x10 (16)
    //   Result: 0xc010_6449
    //
    // Note: some installable copies under include/uapi/drm/amdxdna_accel.h still
    // say DRM_IOW (0x4010_6449). That mismatches the drm_local header the driver
    // includes — Phase 2 must use IOWR like the kernel module.
    // Wrong historical value was 0x4010_6449 (DRM_IOW / dir=1 only).
    #[allow(dead_code)]
    pub const DRM_IOCTL_AMDXDNA_WAIT_CMD: libc::c_ulong = 0xc010_6449;
    // exact: iowr::<amdxdna_drm_wait_cmd>(9) == 0xc010_6449
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
///
/// **Ownership:** Prefer one instance per open `/dev/accel` fd for the process
/// lifetime (owned by `RealXdna2Device` on the Real arm). Creating a second
/// instance after another has already registered DevHeap on the same fd yields
/// EBUSY without a recoverable handle. [`Drop`] munmaps + GEM_CLOSEs the heap.
#[derive(Debug)]
#[allow(dead_code)] // Real implementation scaffolding for the lightweight direct path
pub struct RealDirectSubmission {
    /// The raw fd to the accel device (e.g. /dev/accel/accel0).
    fd: RawFd,
    /// Client-owned DevHeap (handle + mmap). Interior mutability so
    /// `create_hwctx` can ensure the heap once per submission object.
    dev_heap: std::sync::Mutex<Option<DevHeapState>>,
}

impl Drop for RealDirectSubmission {
    fn drop(&mut self) {
        self.release_dev_heap();
    }
}

/// Cloneable handle sharing one [`RealDirectSubmission`] (and its DevHeap) across
/// callers. Prefer this over `RealDirectSubmission::new` on an already-open fd.
#[derive(Clone, Debug)]
pub struct SharedRealDirectSubmission {
    inner: std::sync::Arc<RealDirectSubmission>,
}

impl SharedRealDirectSubmission {
    /// Wrap an existing shared real submission.
    #[must_use]
    pub fn new(inner: std::sync::Arc<RealDirectSubmission>) -> Self {
        Self { inner }
    }

    /// Borrow the underlying real submission.
    #[must_use]
    pub fn as_real(&self) -> &RealDirectSubmission {
        &self.inner
    }
}

impl DirectSubmission for SharedRealDirectSubmission {
    fn create_hwctx(
        &self,
        fd: RawFd,
        num_tiles: u32,
        mem_size: u32,
        qos: Option<&QosInfo>,
    ) -> Result<HardwareContext, DirectError> {
        self.inner.create_hwctx(fd, num_tiles, mem_size, qos)
    }

    fn create_hwctx_with_sizes(
        &self,
        fd: RawFd,
        num_tiles: u32,
        mem_size: u32,
        umq_size: u64,
        log_size: u64,
        qos: Option<&QosInfo>,
    ) -> Result<HardwareContext, DirectError> {
        self.inner
            .create_hwctx_with_sizes(fd, num_tiles, mem_size, umq_size, log_size, qos)
    }

    fn create_hwctx_with_forced_bo_flags(
        &self,
        fd: RawFd,
        num_tiles: u32,
        mem_size: u32,
        qos: Option<&QosInfo>,
        bo_flags: u64,
    ) -> Result<HardwareContext, DirectError> {
        self.inner
            .create_hwctx_with_forced_bo_flags(fd, num_tiles, mem_size, qos, bo_flags)
    }

    fn destroy_hwctx(&self, fd: RawFd, handle: u32) -> Result<(), DirectError> {
        self.inner.destroy_hwctx(fd, handle)
    }

    fn destroy_buffer(&self, fd: RawFd, handle: u32) -> Result<(), DirectError> {
        self.inner.destroy_buffer(fd, handle)
    }

    fn create_buffer(
        &self,
        fd: RawFd,
        size: u64,
        ty: BufferType,
    ) -> Result<BufferHandle, DirectError> {
        self.inner.create_buffer(fd, size, ty)
    }

    fn submit_command(&self, fd: RawFd, cmd: &Command) -> Result<u64, DirectError> {
        self.inner.submit_command(fd, cmd)
    }

    fn wait_command(
        &self,
        fd: RawFd,
        hwctx: u32,
        seq: u64,
        timeout_ms: u32,
    ) -> Result<(), DirectError> {
        self.inner.wait_command(fd, hwctx, seq, timeout_ms)
    }

    fn sync_buffer(
        &self,
        fd: RawFd,
        handle: BufferHandle,
        direction: u32,
        offset: u64,
        size: u64,
    ) -> Result<(), DirectError> {
        self.inner.sync_buffer(fd, handle, direction, offset, size)
    }

    fn get_info(&self, fd: RawFd, param: u32, buffer: &mut [u8]) -> Result<(), DirectError> {
        self.inner.get_info(fd, param, buffer)
    }

    fn get_firmware_version(&self) -> Result<amdxdna_drm_query_firmware_version, DirectError> {
        self.inner.get_firmware_version()
    }

    fn get_power_mode(&self) -> Result<amdxdna_drm_get_power_mode, DirectError> {
        self.inner.get_power_mode()
    }

    fn set_power_mode(&self, mode: u8) -> Result<(), DirectError> {
        self.inner.set_power_mode(mode)
    }

    fn get_resource_info(&self) -> Result<amdxdna_drm_get_resource_info, DirectError> {
        self.inner.get_resource_info()
    }

    fn get_sensors(&self) -> Result<amdxdna_drm_query_sensor, DirectError> {
        self.inner.get_sensors()
    }

    fn get_aie_version(&self) -> Result<amdxdna_drm_query_aie_version, DirectError> {
        self.inner.get_aie_version()
    }

    fn get_bo_info(&self, fd: RawFd, handle: u32) -> Result<amdxdna_drm_get_bo_info, DirectError> {
        self.inner.get_bo_info(fd, handle)
    }

    fn write_bo_bytes(
        &self,
        fd: RawFd,
        handle: u32,
        offset: u64,
        data: &[u8],
    ) -> Result<(), DirectError> {
        self.inner.write_bo_bytes(fd, handle, offset, data)
    }

    fn read_bo_bytes(
        &self,
        fd: RawFd,
        handle: u32,
        offset: u64,
        len: usize,
    ) -> Result<Vec<u8>, DirectError> {
        self.inner.read_bo_bytes(fd, handle, offset, len)
    }

    fn read_bo_bytes_no_sync(
        &self,
        fd: RawFd,
        handle: u32,
        offset: u64,
        len: usize,
    ) -> Result<Vec<u8>, DirectError> {
        self.inner.read_bo_bytes_no_sync(fd, handle, offset, len)
    }

    fn is_real_ioctl_path(&self) -> bool {
        self.inner.is_real_ioctl_path()
    }
}

impl RealDirectSubmission {
    /// Create a real direct submission context for the given open fd.
    /// All future real ioctl work will go through this fd.
    ///
    /// **Do not** construct a second instance for the same fd while the first
    /// still owns DevHeap — reuse one object (or reopen the device).
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

    /// Release the client DevHeap (munmap + GEM_CLOSE) if this instance owns it.
    ///
    /// Safe to call multiple times. Invoked from [`Drop`]. DevHeap is **fd-scoped**
    /// (one per client), not per-submit ephemeral — keep one
    /// [`RealDirectSubmission`] per open accel fd for the process lifetime.
    pub fn release_dev_heap(&self) {
        let state = self.dev_heap.lock().ok().and_then(|mut guard| guard.take());
        if let Some(state) = state {
            if !state.map_ptr.is_null() && state.map_ptr != libc::MAP_FAILED {
                #[allow(unsafe_code)]
                // SAFETY: map_ptr/map_len came from our mmap of this heap BO.
                unsafe {
                    libc::munmap(state.map_ptr, state.map_len);
                }
            }
            let _ = self.destroy_buffer(self.fd, state.handle);
        }
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

            // Pin arg BO handles for the duration of the ioctl (kernel reads
            // userspace array of u32 handles via `args` + `arg_count`).
            let arg_handles = cmd.arg_bo_handles.as_slice();
            let arg_count = u32::try_from(arg_handles.len()).map_err(|_| {
                DirectError::InvalidArgument("arg_bo_handles length exceeds u32".into())
            })?;
            let args_ptr = if arg_handles.is_empty() {
                0u64
            } else {
                arg_handles.as_ptr() as u64
            };

            let mut req = amdxdna_drm_exec_cmd {
                ext: 0,
                ext_flags: 0,
                hwctx: cmd.hwctx_handle,
                ty: cmd.ty as u32,
                cmd_handles: cmd.cmd_bo_handle.map_or(0, u64::from),
                args: args_ptr,
                cmd_count: u32::from(cmd.cmd_bo_handle.is_some()),
                arg_count,
                seq: 0,
            };

            let ret =
                unsafe { ioctl::ioctl(self.fd, ioctl::DRM_IOCTL_AMDXDNA_EXEC_CMD, &raw mut req) };
            // Keep arg_handles live across the ioctl (slice borrows cmd).
            let _ = arg_handles;

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

    /// Wait for a previously submitted EXEC_CMD sequence number.
    ///
    /// Issues `DRM_IOCTL_AMDXDNA_WAIT_CMD`. **Not** gated by
    /// [`exec_cmd_is_enabled`] — cleanup / completion waits must remain
    /// usable; only submit is double-gated.
    ///
    /// `timeout_ms == 0` is infinite in the driver; **refused** unless
    /// `XDNA2_WAIT_ALLOW_INFINITE=1` (lab policy — prefer finite timeouts /
    /// outer `timeout(1)`; see [`submit_and_wait_command`]).
    #[allow(unsafe_code)]
    fn wait_command(
        &self,
        _fd: RawFd,
        hwctx: u32,
        seq: u64,
        timeout_ms: u32,
    ) -> Result<(), DirectError> {
        check_wait_timeout_or_infinite_opt_in(timeout_ms)?;
        #[cfg(feature = "amd-xdna2")]
        {
            let mut req = amdxdna_drm_wait_cmd {
                hwctx,
                timeout: timeout_ms,
                seq,
            };

            let ret =
                unsafe { ioctl::ioctl(self.fd, ioctl::DRM_IOCTL_AMDXDNA_WAIT_CMD, &raw mut req) };

            match ret {
                Ok(_) => Ok(()),
                Err(e) => Err(DirectError::IoctlFailed(format!(
                    "WAIT_CMD failed (hwctx={hwctx}, seq={seq}, timeout_ms={timeout_ms}, \
                     request=0x{:x}): {e}",
                    ioctl::DRM_IOCTL_AMDXDNA_WAIT_CMD
                ))),
            }
        }

        #[cfg(not(feature = "amd-xdna2"))]
        {
            let _ = (hwctx, seq, timeout_ms);
            Err(DirectError::Unsupported(
                "wait_command requires the amd-xdna2 feature".to_string(),
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

    /// Set power mode via `DRM_IOCTL_AMDXDNA_SET_STATE` + `DRM_AMDXDNA_SET_POWER_MODE`.
    ///
    /// Packed from public uAPI (`amdxdna_accel.h`). **Not** automatically exercised
    /// in default unit tests (no `/dev/accel` / no ambient SET). Callers must own
    /// operator policy; does not claim a measured TOPS change.
    fn set_power_mode(&self, mode: u8) -> Result<(), DirectError> {
        if mode > power_mode::TURBO {
            return Err(DirectError::InvalidArgument(format!(
                "set_power_mode: mode={mode} out of range 0..={}",
                power_mode::TURBO
            )));
        }

        #[cfg(feature = "amd-xdna2")]
        {
            let mut payload = amdxdna_drm_set_power_mode {
                power_mode: mode,
                pad: [0; 7],
            };
            let mut req = amdxdna_drm_set_state {
                param: AMDXDNA_SET_PARAM_POWER_MODE,
                #[allow(clippy::cast_possible_truncation)]
                buffer_size: u32::try_from(std::mem::size_of::<amdxdna_drm_set_power_mode>())
                    .unwrap_or(u32::MAX),
                buffer: std::ptr::from_mut(&mut payload).cast::<libc::c_void>() as u64,
            };

            // Safety: payload + req are stack-local; kernel reads buffer of buffer_size.
            #[allow(unsafe_code)]
            let ret =
                unsafe { ioctl::ioctl(self.fd, ioctl::DRM_IOCTL_AMDXDNA_SET_STATE, &raw mut req) };
            match ret {
                Ok(_) => Ok(()),
                Err(e) => Err(DirectError::IoctlFailed(format!(
                    "SET_STATE SET_POWER_MODE mode={mode} failed (request=0x{:x}): {e}",
                    ioctl::DRM_IOCTL_AMDXDNA_SET_STATE
                ))),
            }
        }

        #[cfg(not(feature = "amd-xdna2"))]
        {
            let _ = mode;
            Err(DirectError::Unsupported(
                "set_power_mode requires the amd-xdna2 feature".to_string(),
            ))
        }
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

    /// Host-fill a BO via GET_BO_INFO → mmap → copy → munmap → SYNC_BO(TO_DEVICE).
    ///
    /// **Required** before EXEC_CMD on Cmd BOs: the kernel only sees BO memory,
    /// not [`Command::payload`]. Empty/unfilled Cmd BOs are the 2026-07-13 wedge class.
    #[allow(
        unsafe_code,
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss
    )]
    fn write_bo_bytes(
        &self,
        _fd: RawFd,
        handle: u32,
        offset: u64,
        data: &[u8],
    ) -> Result<(), DirectError> {
        #[cfg(feature = "amd-xdna2")]
        {
            if data.is_empty() {
                return Err(DirectError::InvalidArgument(
                    "write_bo_bytes refuses empty data (empty Cmd BO EXEC_CMD is the \
                     2026-07-13 wedge class; see docs/XDNA2_NEVER_AGAIN.md)"
                        .to_string(),
                ));
            }
            let end = offset.checked_add(data.len() as u64).ok_or_else(|| {
                DirectError::InvalidArgument("write_bo_bytes offset+len overflows u64".to_string())
            })?;
            let info = self.get_bo_info(self.fd, handle)?;
            if info.map_offset == 0 || info.map_offset == u64::MAX {
                return Err(DirectError::IoctlFailed(format!(
                    "write_bo_bytes: handle={handle} has invalid map_offset={:#x}",
                    info.map_offset
                )));
            }
            let map_len = end as usize;
            let map_ptr = unsafe {
                libc::mmap(
                    std::ptr::null_mut(),
                    map_len,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_SHARED,
                    self.fd,
                    info.map_offset as libc::off_t,
                )
            };
            if map_ptr == libc::MAP_FAILED {
                let err = std::io::Error::last_os_error();
                let hint = if err.raw_os_error() == Some(libc::EACCES)
                    || err.raw_os_error() == Some(libc::EPERM)
                {
                    " (hint: open /dev/accel O_RDWR for PROT_WRITE mmap)"
                } else {
                    ""
                };
                return Err(DirectError::IoctlFailed(format!(
                    "mmap BO handle={handle} map_offset={:#x} size={map_len} failed: {err}{hint}",
                    info.map_offset
                )));
            }
            // SAFETY: map_ptr is a successful MAP_SHARED mapping of at least map_len bytes.
            unsafe {
                let dest = map_ptr.cast::<u8>().add(offset as usize);
                std::ptr::copy_nonoverlapping(data.as_ptr(), dest, data.len());
                let _ = libc::munmap(map_ptr, map_len);
            }
            // Direction 0 = TO_DEVICE (amdxdna header / bring-up grid).
            self.sync_buffer(
                self.fd,
                BufferHandle(handle),
                SYNC_BO_TO_DEVICE,
                offset,
                data.len() as u64,
            )?;
            Ok(())
        }

        #[cfg(not(feature = "amd-xdna2"))]
        {
            let _ = (handle, offset, data);
            Err(DirectError::Unsupported(
                "write_bo_bytes requires the amd-xdna2 feature".to_string(),
            ))
        }
    }

    /// Read host-visible BO bytes via SYNC FROM_DEVICE → mmap → copy.
    ///
    /// Used for general reads. [`timed_bo_roundtrip_direct`] uses
    /// [`Self::read_bo_bytes_no_sync`] after an exclusive SYNC FROM for clean timings.
    fn read_bo_bytes(
        &self,
        _fd: RawFd,
        handle: u32,
        offset: u64,
        len: usize,
    ) -> Result<Vec<u8>, DirectError> {
        if len == 0 {
            return Err(DirectError::InvalidArgument(
                "read_bo_bytes refuses len=0".to_string(),
            ));
        }
        self.sync_buffer(
            self.fd,
            BufferHandle(handle),
            SYNC_BO_FROM_DEVICE,
            offset,
            len as u64,
        )?;
        self.read_bo_bytes_no_sync(self.fd, handle, offset, len)
    }

    /// mmap-copy BO contents without SYNC (caller must ensure host coherency).
    #[allow(
        unsafe_code,
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss
    )]
    fn read_bo_bytes_no_sync(
        &self,
        _fd: RawFd,
        handle: u32,
        offset: u64,
        len: usize,
    ) -> Result<Vec<u8>, DirectError> {
        #[cfg(feature = "amd-xdna2")]
        {
            if len == 0 {
                return Err(DirectError::InvalidArgument(
                    "read_bo_bytes_no_sync refuses len=0".to_string(),
                ));
            }
            let end = offset.checked_add(len as u64).ok_or_else(|| {
                DirectError::InvalidArgument(
                    "read_bo_bytes_no_sync offset+len overflows u64".to_string(),
                )
            })?;
            let info = self.get_bo_info(self.fd, handle)?;
            if info.map_offset == 0 || info.map_offset == u64::MAX {
                return Err(DirectError::IoctlFailed(format!(
                    "read_bo_bytes_no_sync: handle={handle} has invalid map_offset={:#x}",
                    info.map_offset
                )));
            }
            let map_len = end as usize;
            let map_ptr = unsafe {
                libc::mmap(
                    std::ptr::null_mut(),
                    map_len,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_SHARED,
                    self.fd,
                    info.map_offset as libc::off_t,
                )
            };
            if map_ptr == libc::MAP_FAILED {
                let err = std::io::Error::last_os_error();
                return Err(DirectError::IoctlFailed(format!(
                    "mmap BO handle={handle} map_offset={:#x} size={map_len} for read failed: {err}",
                    info.map_offset
                )));
            }
            let mut out = vec![0u8; len];
            // SAFETY: map_ptr is a successful MAP_SHARED mapping of at least map_len bytes.
            unsafe {
                let src = map_ptr.cast::<u8>().add(offset as usize);
                std::ptr::copy_nonoverlapping(src, out.as_mut_ptr(), len);
                let _ = libc::munmap(map_ptr, map_len);
            }
            Ok(out)
        }

        #[cfg(not(feature = "amd-xdna2"))]
        {
            let _ = (handle, offset, len);
            Err(DirectError::Unsupported(
                "read_bo_bytes_no_sync requires the amd-xdna2 feature".to_string(),
            ))
        }
    }

    fn is_real_ioctl_path(&self) -> bool {
        true
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

    /// BO handles for `amdxdna_drm_exec_cmd.args` / `arg_count` (IFM, WTS, OFM,
    /// instruction, …). Empty = historical `arg_count=0` behavior.
    ///
    /// XRT pins bound arg BOs here in addition to packing paddrs into START_NPU
    /// kernel args. Lab OFM path populates this when `XDNA2_LAB_OFM=1`.
    pub arg_bo_handles: Vec<u32>,
}

// =============================================================================
// EXEC_CMD payload builders (Phase 1) + timed op API
// =============================================================================
//
// Provenance (GPL-2.0 amd/xdna-driver + public uAPI — docs/XDNA2_CMD_PAYLOAD.md):
// - amdxdna_ctx.h: packed amdxdna_cmd header, ERT opcodes/states, START_NPU/CHAIN
// - amdxdna_accel.h: EXEC_CMD / WAIT_CMD ioctl structs
// - Instruction / txn control-code bytes remain firmware-opaque
//
// Building bytes is always safe. Submitting requires exec_cmd_is_enabled() (+ L3).

/// ERT command opcodes (`enum ert_cmd_opcode` in amdxdna_ctx.h).
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErtCmdOpcode {
    /// Invalid / unset (`ERT_INVALID_CMD`).
    Invalid = 0xffff_ffff,
    /// Start compute unit (legacy ERT).
    StartCu = 0,
    /// Start DPU (`ERT_START_DPU` = 18).
    StartDpu = 18,
    /// Chain of command BOs (`ERT_CMD_CHAIN` = 19).
    CmdChain = 19,
    /// Start NPU with instruction buffer (`ERT_START_NPU` = 20).
    StartNpu = 20,
    /// Start NPU with preemption buffers (`ERT_START_NPU_PREEMPT` = 21).
    StartNpuPreempt = 21,
    /// Start NPU preempt ELF path (`ERT_START_NPU_PREEMPT_ELF` = 22).
    StartNpuPreemptElf = 22,
}

/// ERT command state (`enum ert_cmd_state`).
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErtCmdState {
    /// Command newly created / ready to submit.
    New = 1,
    /// Queued in scheduler.
    Queued = 2,
    /// Running on hardware.
    Running = 3,
    /// Hardware completed successfully.
    Completed = 4,
    /// Hardware reported error.
    Error = 5,
    /// Aborted.
    Abort = 6,
    /// Submitted to firmware.
    Submitted = 7,
    /// Timed out (may carry health data in payload).
    Timeout = 8,
    /// No response from firmware.
    NoResponse = 9,
}

/// Unpacked view of the packed `amdxdna_cmd.header` dword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmdxdnaCmdHeaderFields {
    /// Bits 3:0 — ERT state.
    pub state: u32,
    /// Bits 11:10 — extra CU mask count beyond the first.
    pub extra_cu_mask: u32,
    /// Bits 22:12 — count field (11 bits).
    pub count: u32,
    /// Bits 27:23 — ERT opcode (5 bits).
    pub opcode: u32,
}

/// Pack STATE / EXTRA_CU_MASK / COUNT / OPCODE into one LE `u32` header dword.
///
/// Matches Linux `FIELD_PREP` on:
/// `AMDXDNA_CMD_STATE`, `EXTRA_CU_MASK`, `COUNT`, `OPCODE` masks.
///
/// Field widths (overflow is truncated to fit):
/// - `state`: 4 bits (`0..=15`)
/// - `extra_cu_mask`: 2 bits (`0..=3`)
/// - `count`: 11 bits (`0..=0x7ff`) — total `data[]` dword count after header
/// - `opcode`: 5 bits (`0..=31`) — note `ERT_INVALID_CMD` (`!0`) is **not**
///   representable; packing it yields `0x1f`
#[must_use]
pub fn pack_amdxdna_cmd_header(state: u32, extra_cu_mask: u32, count: u32, opcode: u32) -> u32 {
    (state & 0xf)
        | ((extra_cu_mask & 0x3) << 10)
        | ((count & 0x7ff) << 12)
        | ((opcode & 0x1f) << 23)
}

/// Unpack a packed `amdxdna_cmd.header` dword.
#[must_use]
pub fn unpack_amdxdna_cmd_header(header: u32) -> AmdxdnaCmdHeaderFields {
    AmdxdnaCmdHeaderFields {
        state: header & 0xf,
        extra_cu_mask: (header >> 10) & 0x3,
        count: (header >> 12) & 0x7ff,
        opcode: (header >> 23) & 0x1f,
    }
}

/// Parse packed header from the first 4 **little-endian** bytes of a Cmd BO.
///
/// Returns [`DirectError::InvalidArgument`] if fewer than 4 bytes are present.
/// **Does not** validate that the remainder of the buffer matches `count`.
pub fn parse_amdxdna_cmd_header_from_bytes(
    bytes: &[u8],
) -> Result<AmdxdnaCmdHeaderFields, DirectError> {
    if bytes.len() < 4 {
        return Err(DirectError::InvalidArgument(format!(
            "Cmd BO too short for amdxdna_cmd.header: need 4 bytes, got {}",
            bytes.len()
        )));
    }
    let header = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    Ok(unpack_amdxdna_cmd_header(header))
}

/// Number of leading CU-mask dwords in `amdxdna_cmd.data[]`.
///
/// Matches `amdxdna_cmd_get_payload`: **0** for `ERT_CMD_CHAIN`, else
/// `1 + EXTRA_CU_MASK`.
#[must_use]
pub fn amdxdna_cmd_num_cu_masks(opcode: u32, extra_cu_mask: u32) -> u32 {
    if opcode == ErtCmdOpcode::CmdChain as u32 {
        0
    } else {
        1u32.saturating_add(extra_cu_mask & 0x3)
    }
}

/// Build a full Cmd BO byte buffer: packed header + `data` dwords (LE).
///
/// **Does not submit.** Layout matches `struct amdxdna_cmd { u32 header; u32 data[]; }`.
#[must_use]
pub fn build_amdxdna_cmd_bo(header: u32, data_dwords: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + data_dwords.len().saturating_mul(4));
    out.extend_from_slice(&header.to_le_bytes());
    for d in data_dwords {
        out.extend_from_slice(&d.to_le_bytes());
    }
    out
}

/// Maximum value of the packed `amdxdna_cmd` COUNT field (11 bits).
pub const AMDXDNA_CMD_COUNT_MAX: u32 = 0x7ff;

/// Fail-closed if `data[]` dword count cannot fit in the 11-bit COUNT field.
///
/// Prevents silent header under-report when builders emit a longer body than
/// COUNT can describe (`amdxdna_cmd_get_payload` trusts COUNT).
fn ensure_cmd_data_dword_count(count: u32) -> Result<u32, DirectError> {
    if count > AMDXDNA_CMD_COUNT_MAX {
        return Err(DirectError::InvalidArgument(format!(
            "amdxdna_cmd COUNT overflow: data dwords {count} > max {AMDXDNA_CMD_COUNT_MAX} (11-bit field)"
        )));
    }
    Ok(count)
}

/// Build an `ERT_START_NPU` Cmd BO envelope (instruction buffer address + size).
///
/// Layout after header (AIE2 primary path — `aie2_cmdlist_fill_*` uses this):
/// 1. **one** CU mask dword (`extra_cu_mask = 0`; non-zero mask required for a
///    valid CU index via `ffs` in the driver)
/// 2. `amdxdna_cmd_start_npu`: u64 `buffer`, u32 `buffer_size`, u32 `prop_count`,
///    `prop_args[]`
///
/// # Footgun / prefer kernel-args builder for I/O
///
/// **Legacy property packing:** this entry treats **all** of `prop_args` as
/// properties (`prop_count = prop_args.len()`). That is correct only for real
/// property dwords or **empty** envelopes (unit tests / structural probes).
///
/// For **kernel args** (IFM/WTS/OFM paddrs) XRT shim_test always uses
/// `prop_count = 0` while still appending paddrs in the flexible array. Passing
/// those paddrs here marks every dword as properties and **misbinds** firmware
/// I/O — use [`build_ert_start_npu_cmd_bo_with_kernel_args`] (or
/// [`build_ert_start_npu_cmd_bo_ex`] with explicit `prop_count`) instead.
///
/// Lab timed path (`XDNA2_LAB_OFM=1`) uses `_with_kernel_args` only.
///
/// `COUNT` = total number of `data[]` dwords (CU masks + payload), matching
/// `amdxdna_cmd_get_payload` (`payload_size = (count - num_masks) * 4`).
/// Returns [`DirectError::InvalidArgument`] if COUNT would exceed 11 bits.
///
/// `inst_buffer_addr` must be a **device-visible** instruction address for real
/// compute. Zero is allowed only for structural tests — not a known-good job.
/// Without valid **txn control code** at that address, this is still not compute.
///
/// `cu_mask0`: use a non-zero mask for any buffer intended for driver fill
/// (`ffs` → CU index). Zero is allowed **only** for structural byte-layout
/// tests; the driver will reject all-zero masks with `-EINVAL`.
///
/// **Does not submit.**
pub fn build_ert_start_npu_cmd_bo(
    inst_buffer_addr: u64,
    inst_buffer_size: u32,
    prop_args: &[u32],
    cu_mask0: u32,
) -> Result<Vec<u8>, DirectError> {
    let prop_count = u32::try_from(prop_args.len())
        .map_err(|_| DirectError::InvalidArgument("prop_args length exceeds u32".into()))?;
    build_ert_start_npu_cmd_bo_ex(
        inst_buffer_addr,
        inst_buffer_size,
        prop_count,
        prop_args,
        cu_mask0,
    )
}

/// Build `ERT_START_NPU` with explicit `prop_count` separate from flexible-array length.
///
/// Driver contract (`amdxdna_cmd_start_npu`):
/// - `prop_count` = number of leading **property** dwords
/// - remaining `prop_args[prop_count..]` = **kernel args** (e.g. BO paddrs)
/// - firmware `arg_cnt` = full flexible-array dword count
///
/// XRT shim_test always uses `prop_count = 0` for START_NPU I/O bind.
///
/// # Errors
///
/// `prop_count > prop_args.len()`, COUNT overflow, or u32 conversion failure.
///
/// **Does not submit.**
pub fn build_ert_start_npu_cmd_bo_ex(
    inst_buffer_addr: u64,
    inst_buffer_size: u32,
    prop_count: u32,
    prop_args: &[u32],
    cu_mask0: u32,
) -> Result<Vec<u8>, DirectError> {
    debug_assert_ne!(
        cu_mask0, 0,
        "cu_mask0=0 is invalid for driver fill (INVALID_CU_IDX); use only in structural tests"
    );
    let args_len = u32::try_from(prop_args.len())
        .map_err(|_| DirectError::InvalidArgument("prop_args length exceeds u32".into()))?;
    if prop_count > args_len {
        return Err(DirectError::InvalidArgument(format!(
            "build_ert_start_npu_cmd_bo_ex: prop_count={prop_count} > prop_args.len()={args_len}"
        )));
    }
    // COUNT: number of data dwords after header (1 cu_mask + start_npu fixed + args).
    let data_dword_count = ensure_cmd_data_dword_count(
        1u32 // cu mask
            .saturating_add(2) // buffer u64
            .saturating_add(1) // buffer_size
            .saturating_add(1) // prop_count field
            .saturating_add(args_len),
    )?;
    let header = pack_amdxdna_cmd_header(
        ErtCmdState::New as u32,
        0, // extra_cu_mask: single mask dword
        data_dword_count,
        ErtCmdOpcode::StartNpu as u32,
    );
    let mut data = Vec::with_capacity(data_dword_count as usize);
    data.push(cu_mask0);
    // Split device address into LE dwords (intentional low/high 32-bit halves).
    data.push((inst_buffer_addr & 0xffff_ffff) as u32);
    data.push((inst_buffer_addr >> 32) as u32);
    data.push(inst_buffer_size);
    data.push(prop_count);
    data.extend_from_slice(prop_args);
    debug_assert_eq!(data.len(), data_dword_count as usize);
    Ok(build_amdxdna_cmd_bo(header, &data))
}

/// Build `ERT_START_NPU` with **`prop_count = 0`** and kernel-arg dwords only.
///
/// Preferred for lab OFM bind (shim-compatible): IFM/WTS/OFM paddrs live in the
/// flexible array as kernel args, not properties. See
/// [`super::lab_io::pack_start_npu_kernel_args`].
///
/// **Does not submit.**
pub fn build_ert_start_npu_cmd_bo_with_kernel_args(
    inst_buffer_addr: u64,
    inst_buffer_size: u32,
    kernel_args: &[u32],
    cu_mask0: u32,
) -> Result<Vec<u8>, DirectError> {
    build_ert_start_npu_cmd_bo_ex(inst_buffer_addr, inst_buffer_size, 0, kernel_args, cu_mask0)
}

/// Build an `ERT_START_NPU_PREEMPT` Cmd BO envelope (`amdxdna_cmd_preempt_data`).
///
/// Layout after header:
/// 1. one CU mask dword
/// 2. `inst_buf` / `save_buf` / `restore_buf` (u64 each)
/// 3. `inst_size` / `save_size` / `restore_size` / `inst_prop_cnt` (u32 each)
/// 4. `prop_args[]`
///
/// Same fixed prefix is used for `ERT_START_NPU_PREEMPT_ELF` in the driver
/// (`aie2_cmdlist_fill_npu_elf`); only the opcode differs. Pass
/// [`ErtCmdOpcode::StartNpuPreempt`] or [`ErtCmdOpcode::StartNpuPreemptElf`]
/// only — any other opcode returns [`DirectError::InvalidArgument`].
///
/// AIE2 requires the preempt feature flag; otherwise fill returns `-EOPNOTSUPP`.
/// **Does not submit.** Instruction/save/restore **contents** remain opaque.
///
/// `cu_mask0`: non-zero required for driver fill; zero only for structural tests.
#[allow(clippy::too_many_arguments)] // mirrors fixed `amdxdna_cmd_preempt_data` fields
pub fn build_ert_start_npu_preempt_cmd_bo(
    inst_buf: u64,
    save_buf: u64,
    restore_buf: u64,
    inst_size: u32,
    save_size: u32,
    restore_size: u32,
    prop_args: &[u32],
    cu_mask0: u32,
    opcode: ErtCmdOpcode,
) -> Result<Vec<u8>, DirectError> {
    debug_assert_ne!(
        cu_mask0, 0,
        "cu_mask0=0 is invalid for driver fill (INVALID_CU_IDX); use only in structural tests"
    );
    let opcode_u32 = match opcode {
        ErtCmdOpcode::StartNpuPreempt | ErtCmdOpcode::StartNpuPreemptElf => opcode as u32,
        other => {
            return Err(DirectError::InvalidArgument(format!(
                "build_ert_start_npu_preempt_cmd_bo requires StartNpuPreempt or \
                 StartNpuPreemptElf, got {other:?}"
            )));
        }
    };
    let prop_count = u32::try_from(prop_args.len())
        .map_err(|_| DirectError::InvalidArgument("prop_args length exceeds u32".into()))?;
    // 1 cu + 6 for three u64s + 4 fixed u32s + props
    let data_dword_count = ensure_cmd_data_dword_count(
        1u32.saturating_add(6)
            .saturating_add(4)
            .saturating_add(prop_count),
    )?;
    let header = pack_amdxdna_cmd_header(ErtCmdState::New as u32, 0, data_dword_count, opcode_u32);
    let mut data = Vec::with_capacity(data_dword_count as usize);
    data.push(cu_mask0);
    data.push((inst_buf & 0xffff_ffff) as u32);
    data.push((inst_buf >> 32) as u32);
    data.push((save_buf & 0xffff_ffff) as u32);
    data.push((save_buf >> 32) as u32);
    data.push((restore_buf & 0xffff_ffff) as u32);
    data.push((restore_buf >> 32) as u32);
    data.push(inst_size);
    data.push(save_size);
    data.push(restore_size);
    data.push(prop_count); // inst_prop_cnt
    data.extend_from_slice(prop_args);
    debug_assert_eq!(data.len(), data_dword_count as usize);
    Ok(build_amdxdna_cmd_bo(header, &data))
}

/// Convenience: `ERT_START_NPU_PREEMPT_ELF` with the same fixed prefix as preempt.
#[allow(clippy::too_many_arguments)]
pub fn build_ert_start_npu_preempt_elf_cmd_bo(
    inst_buf: u64,
    save_buf: u64,
    restore_buf: u64,
    inst_size: u32,
    save_size: u32,
    restore_size: u32,
    prop_args: &[u32],
    cu_mask0: u32,
) -> Result<Vec<u8>, DirectError> {
    build_ert_start_npu_preempt_cmd_bo(
        inst_buf,
        save_buf,
        restore_buf,
        inst_size,
        save_size,
        restore_size,
        prop_args,
        cu_mask0,
        ErtCmdOpcode::StartNpuPreemptElf,
    )
}

/// Build an `ERT_START_DPU` Cmd BO from `struct amdxdna_cmd_start_dpu`.
///
/// Fixed payload (after one CU mask):
/// - `dtrace_buffer` u64
/// - `instruction_buffer` u64
/// - `instruction_buffer_size` u32
/// - `uc_index` u16 + `chained` u16 (packed LE: low=`uc_index`, high=`chained`)
///
/// **This builder only supports a single DPU descriptor** (`chained == 0`).
/// A non-zero `chained` means “N following `amdxdna_cmd_start_dpu` elements” in
/// the kernel header; emitting only one element would be an incomplete series.
/// Returns [`DirectError::InvalidArgument`] if `chained != 0`.
///
/// **Target note (honest):** AIE2/XDNA2 `aie2_cmdlist_fill_slot` does **not**
/// handle `ERT_START_DPU` (returns `-EOPNOTSUPP`). AIE2 compute uses
/// [`build_ert_start_npu_cmd_bo`] (`ERT_START_NPU`). This builder matches the
/// public header for AIE4/VE2 paths and offline layout tests only.
///
/// `cu_mask0`: non-zero required for driver fill; zero only for structural tests.
///
/// **Does not submit.**
pub fn build_ert_start_dpu_cmd_bo(
    dtrace_buffer: u64,
    instruction_buffer: u64,
    instruction_buffer_size: u32,
    uc_index: u16,
    chained: u16,
    cu_mask0: u32,
) -> Result<Vec<u8>, DirectError> {
    debug_assert_ne!(
        cu_mask0, 0,
        "cu_mask0=0 is invalid for driver fill (INVALID_CU_IDX); use only in structural tests"
    );
    if chained != 0 {
        return Err(DirectError::InvalidArgument(format!(
            "build_ert_start_dpu_cmd_bo only supports chained==0 (single descriptor); \
             got chained={chained}. Multi-element DPU series not implemented."
        )));
    }
    // 1 cu + 2 dtrace + 2 inst + size + packed uc/chained = 7 data dwords
    let data_dword_count = ensure_cmd_data_dword_count(7)?;
    let header = pack_amdxdna_cmd_header(
        ErtCmdState::New as u32,
        0,
        data_dword_count,
        ErtCmdOpcode::StartDpu as u32,
    );
    let mut data = Vec::with_capacity(data_dword_count as usize);
    data.push(cu_mask0);
    data.push((dtrace_buffer & 0xffff_ffff) as u32);
    data.push((dtrace_buffer >> 32) as u32);
    data.push((instruction_buffer & 0xffff_ffff) as u32);
    data.push((instruction_buffer >> 32) as u32);
    data.push(instruction_buffer_size);
    // LE packing of two u16 fields in one dword (chained is always 0 here)
    data.push(u32::from(uc_index) | (u32::from(chained) << 16));
    debug_assert_eq!(data.len(), data_dword_count as usize);
    Ok(build_amdxdna_cmd_bo(header, &data))
}

/// Build an `ERT_CMD_CHAIN` Cmd BO from nested command BO handles (`u64` each).
///
/// No CU masks before the chain body (`amdxdna_cmd_get_payload` uses
/// `num_masks = 0` for this opcode).
///
/// Empty `cmd_handles` produces a structural envelope only; the driver chain
/// helper rejects `command_count == 0` / `> 64` on submit paths.
/// Returns [`DirectError::InvalidArgument`] if COUNT would exceed 11 bits.
///
/// **Does not submit.**
pub fn build_ert_cmd_chain_bo(cmd_handles: &[u64]) -> Result<Vec<u8>, DirectError> {
    let command_count = u32::try_from(cmd_handles.len())
        .map_err(|_| DirectError::InvalidArgument("cmd_handles length exceeds u32".into()))?;
    // chain header: command_count, submit_index, error_index, reserved[3] = 6 u32
    // + 2 u32 per handle
    let data_dword_count =
        ensure_cmd_data_dword_count(6u32.saturating_add(command_count.saturating_mul(2)))?;
    let header = pack_amdxdna_cmd_header(
        ErtCmdState::New as u32,
        0,
        data_dword_count,
        ErtCmdOpcode::CmdChain as u32,
    );
    let mut data = Vec::with_capacity(data_dword_count as usize);
    data.push(command_count);
    data.push(0); // submit_index
    data.push(0); // error_index
    data.push(0);
    data.push(0);
    data.push(0);
    for h in cmd_handles {
        data.push((*h & 0xffff_ffff) as u32);
        data.push((*h >> 32) as u32);
    }
    debug_assert_eq!(data.len(), data_dword_count as usize);
    Ok(build_amdxdna_cmd_bo(header, &data))
}

/// Legacy research scaffold (pre-Phase-1).
///
/// Historically wrote four separate u32 words (state, opcode, count, reserved)
/// instead of the **packed** `amdxdna_cmd.header`. Prefer typed builders:
/// [`build_ert_start_npu_cmd_bo`], [`build_ert_cmd_chain_bo`],
/// [`build_ert_start_npu_preempt_cmd_bo`], [`build_ert_start_dpu_cmd_bo`], or
/// [`build_amdxdna_cmd_bo`].
///
/// Now builds a **packed** header with opcode 0 / state NEW and `body_dwords`
/// zero data words so length tests stay meaningful. Body length is **clamped**
/// to [`AMDXDNA_CMD_COUNT_MAX`] so COUNT always matches the emitted `data[]`
/// (no silent header/body mismatch). Still **not** a valid compute job —
/// never submit this to hardware.
#[must_use]
pub fn build_exec_cmd_payload_scaffold(body_dwords: usize) -> Vec<u8> {
    let count = u32::try_from(body_dwords)
        .unwrap_or(AMDXDNA_CMD_COUNT_MAX)
        .min(AMDXDNA_CMD_COUNT_MAX);
    let header = pack_amdxdna_cmd_header(ErtCmdState::New as u32, 0, count, 0);
    // Clamp body to COUNT so packed header and buffer stay consistent.
    let zeros = vec![0u32; count as usize];
    build_amdxdna_cmd_bo(header, &zeros)
}

/// Deprecated loose header fields for older call sites; prefer packed helpers.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ExecCmdPayloadHeader {
    /// Prefer [`pack_amdxdna_cmd_header`] (was incorrectly a free u32, not bitfield).
    pub state: u32,
    /// Prefer packed opcode field in `amdxdna_cmd.header`.
    pub opcode: u32,
    /// Prefer packed count field in `amdxdna_cmd.header`.
    pub count: u32,
    /// Unused padding from the legacy four-dword layout.
    pub reserved: u32,
}

impl Default for ExecCmdPayloadHeader {
    fn default() -> Self {
        Self {
            state: ErtCmdState::New as u32,
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

/// Default finite WAIT_CMD timeout (ms) for lab single-shot helpers.
///
/// Override with env `XDNA2_WAIT_TIMEOUT_MS`. Infinite wait (`0`) is refused by
/// [`submit_and_wait_command`] (lab policy — never hang forever).
pub const DEFAULT_WAIT_CMD_TIMEOUT_MS: u32 = 5_000;

/// SYNC_BO direction: host → device (uAPI / bring-up grid uses `0`).
pub const SYNC_BO_TO_DEVICE: u32 = 0;

/// SYNC_BO direction: device → host (uAPI / bring-up grid uses `1`).
pub const SYNC_BO_FROM_DEVICE: u32 = 1;

/// Max elements (`m * n * k`) for optional host golden matmul (safety cap).
///
/// Shared with [`super::numerical::host_matmul_f32_ones_vec`] so refuse thresholds
/// cannot drift.
pub const HOST_REF_MAX_ELEMENTS: u64 = 4_000_000;

/// Max instruction/txn blob size loaded from path or hex env (64 MiB).
///
/// Defense-in-depth against accidental multi-GB paths before full `fs::read`.
pub const MAX_INSTRUCTION_BLOB_BYTES: u64 = 64 * 1024 * 1024;

/// Default Shmem size for [`timed_bo_roundtrip_direct`] (bytes).
pub const DEFAULT_BO_ROUNDTRIP_SIZE: u64 = 4096;

/// Pure lab policy for finite WAIT timeouts (no env / no ioctl).
///
/// Returns `Ok(())` when `timeout_ms > 0`. `0` is infinite in the driver and is
/// refused here so unit tests cover the policy without opening L1 gates or
/// `/dev/accel`. Used by [`submit_and_wait_command`].
///
/// # Errors
///
/// [`DirectError::InvalidArgument`] when `timeout_ms == 0`.
pub fn check_finite_wait_timeout(timeout_ms: u32) -> Result<(), DirectError> {
    if timeout_ms == 0 {
        return Err(DirectError::InvalidArgument(
            "timeout_ms=0 is infinite WAIT_CMD; pass a finite timeout in milliseconds \
             (lab policy — see docs/XDNA2_EXEC_CMD_LAB.md)"
                .to_string(),
        ));
    }
    Ok(())
}

/// Whether infinite WAIT (`timeout_ms == 0`) is explicitly opted in.
///
/// Env `XDNA2_WAIT_ALLOW_INFINITE=1` (or `true`). Default: **false**.
#[must_use]
pub fn wait_infinite_is_allowed() -> bool {
    std::env::var("XDNA2_WAIT_ALLOW_INFINITE")
        .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

/// Gate for raw [`DirectSubmission::wait_command`]: finite timeout, or infinite
/// only with [`wait_infinite_is_allowed`].
///
/// # Errors
///
/// [`DirectError::InvalidArgument`] when `timeout_ms == 0` without opt-in.
pub fn check_wait_timeout_or_infinite_opt_in(timeout_ms: u32) -> Result<(), DirectError> {
    if timeout_ms == 0 && !wait_infinite_is_allowed() {
        return Err(DirectError::InvalidArgument(
            "wait_command refuses timeout_ms=0 (infinite) unless XDNA2_WAIT_ALLOW_INFINITE=1; \
             prefer finite ms or outer timeout(1) — see docs/XDNA2_EXEC_CMD_LAB.md"
                .to_string(),
        ));
    }
    Ok(())
}

/// Host-fill a Cmd BO with envelope/payload bytes before EXEC_CMD.
///
/// Calls [`DirectSubmission::write_bo_bytes`] (Real: GET_BO_INFO → mmap → copy →
/// SYNC_BO TO_DEVICE). Refuses empty payloads (empty Cmd BO is the 2026-07-13
/// wedge class). **Required** before [`submit_and_wait_command`] on silicon.
///
/// # Errors
///
/// - [`DirectError::InvalidArgument`] for empty `payload`
/// - mmap / GET_BO_INFO / SYNC failures from the backend
pub fn fill_cmd_bo_with_payload(
    submission: &dyn DirectSubmission,
    fd: RawFd,
    handle: u32,
    payload: &[u8],
) -> Result<(), DirectError> {
    if payload.is_empty() {
        return Err(DirectError::InvalidArgument(
            "fill_cmd_bo_with_payload refuses empty payload; empty Cmd BO EXEC_CMD is the \
             2026-07-13 wedge class (see docs/XDNA2_NEVER_AGAIN.md / XDNA2_EXEC_CMD_LAB.md)"
                .to_string(),
        ));
    }
    submission.write_bo_bytes(fd, handle, 0, payload)
}

/// Result of a gated EXEC_CMD + WAIT_CMD pair ([`submit_and_wait_command`]).
///
/// **Honesty:** `seq` only proves the driver accepted the ioctl and wait
/// returned. It does **not** prove real NPU compute or a valid txn payload.
/// ERT state from a mapped Cmd BO is a documented follow-up (not filled here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitWaitResult {
    /// Sequence number returned by EXEC_CMD (passed to WAIT_CMD).
    pub seq: u64,
    /// Timeout in milliseconds that was requested for WAIT_CMD.
    pub timeout_ms: u32,
}

/// Submit one EXEC_CMD then WAIT_CMD with a **finite** timeout.
///
/// **Safety / NEVER AGAIN:**
/// 1. Refuses unless [`exec_cmd_is_enabled`] (feature + `XDNA2_ALLOW_EXEC_CMD=1`).
/// 2. Refuses `timeout_ms == 0` (infinite wait) — pass a positive ms value.
/// 3. Does **not** loop on failure. Caller must not retry in a tight loop.
/// 4. Caller **must** host-fill the Cmd BO first ([`fill_cmd_bo_with_payload`] +
///    Phase 1 builders). `Command.payload` alone is **not** written by submit.
/// 5. Lab kernel L3 may still return `-EPERM` until `exec_cmd_allow=1`.
///
/// See **`docs/XDNA2_EXEC_CMD_LAB.md`**. Does **not** set `ran_on_npu`; does not
/// claim silicon compute success from envelope-only payloads.
///
/// # Errors
///
/// - [`DirectError::Unsupported`] when L1 gates are closed.
/// - [`DirectError::InvalidArgument`] when `timeout_ms == 0`.
/// - [`DirectError::IoctlFailed`] / other errors from submit or wait.
pub fn submit_and_wait_command(
    submission: &dyn DirectSubmission,
    fd: RawFd,
    cmd: &Command,
    timeout_ms: u32,
) -> Result<SubmitWaitResult, DirectError> {
    if !exec_cmd_is_enabled() {
        return Err(DirectError::Unsupported(
            exec_cmd_block_reason().to_string(),
        ));
    }
    check_finite_wait_timeout(timeout_ms)?;

    let seq = submission.submit_command(fd, cmd)?;
    submission.wait_command(fd, cmd.hwctx_handle, seq, timeout_ms)?;
    Ok(SubmitWaitResult { seq, timeout_ms })
}

/// Result of a timed buffer / matmul attempt on the direct path.
///
/// Designed so colibrì tools can parse the same fields whether the op ran on
/// silicon or returned an honest skip/error.
///
/// **Honesty (Phase 3):**
/// - `ran_on_npu` is **never** set true for envelope-only START_NPU or instruction-blob
///   submit without numerical verify (see `docs/XDNA2_DIRECT_CAPABILITIES.md`).
/// - `ran_on_device_path` is **only** for real DRM ioctl DMA ([`timed_bo_roundtrip_direct`]
///   on [`RealDirectSubmission`]), **not** stub in-memory verify and **not** GEMM.
/// - `host_ref_*` is always host CPU math, never NPU.
/// - Stage fields are exclusive where possible: `total_s` is wall clock; do not sum
///   stages expecting equality with `total_s` (overlap/gaps remain possible).
/// - For `op == "bo_roundtrip"`, matrix dims are unused (`m`/`n`/`k` = 0); use
///   [`Self::size_bytes`] for buffer size.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)] // honesty flags are intentional product fields
pub struct TimedOpResult {
    /// Logical op name (`"matmul"`, `"bo_roundtrip"`, …).
    pub op: String,
    /// Matrix M dimension (rows of A / output); **0** for non-matmul ops.
    pub m: u32,
    /// Matrix N dimension (cols of B / output); 0 if N/A.
    pub n: u32,
    /// Matrix K dimension (inner); 0 if N/A.
    pub k: u32,
    /// Buffer size in bytes for BO ops (`bo_roundtrip`); `None` for matmul.
    pub size_bytes: Option<u64>,
    /// Dtype string (`"f32"`, `"bf16"`, `"bytes"`, …).
    pub dtype: String,
    /// Wall time for the whole attempt (host clock), seconds.
    pub total_s: f64,
    /// Host-only prep (create + host fill), seconds — excludes device SYNC/submit.
    pub host_prep_s: f64,
    /// CREATE_HWCTX stage wall time, seconds (`None` if not attempted).
    pub hwctx_s: Option<f64>,
    /// Host fill stage (write_bo / Cmd BO fill), seconds — excludes SYNC FROM / wait.
    pub fill_s: Option<f64>,
    /// submit_and_wait wall time when L1 open and attempted, seconds.
    pub submit_wait_s: Option<f64>,
    /// Device-side time: SYNC BO (roundtrip) or submit_and_wait (matmul), seconds.
    pub device_s: Option<f64>,
    /// Host golden matmul wall time when computed (`None` if skipped).
    pub host_ref_s: Option<f64>,
    /// Host golden matmul succeeded (CPU only; never NPU).
    pub host_ref_ok: bool,
    /// Host golden detail (checksum / first element / skip reason).
    pub host_ref_detail: String,
    /// Whether **verified NPU compute** completed successfully.
    ///
    /// True **only** after lab OFM readback +
    /// [`super::numerical::decide_ran_on_npu`] (non-empty golden/candidate,
    /// real ioctl, instruction blob, submit/wait ok, allclose) when
    /// `XDNA2_LAB_OFM=1`. Default timed path (LAB_OFM off) is hard-false.
    /// Instruction-blob submit alone is **not** enough. Statuses
    /// `ok_submit_wait_envelope` / `ok_submit_wait_with_inst_blob` keep this
    /// false; product Pass uses `ok_npu_verified` only after decide.
    pub ran_on_npu: bool,
    /// True **only** when real DRM ioctls completed a verified BO DMA path.
    /// Never true for [`StubDirectSubmission`] in-memory roundtrips.
    /// **Not** NPU compute Pass — do not map to `timed_gemm` Pass.
    pub ran_on_device_path: bool,
    /// True when the submission backend is real ioctl (`RealDirectSubmission`).
    pub real_ioctl: bool,
    /// Stable machine status: `hwctx_ok`, `ok_submit_wait_envelope`,
    /// `ok_submit_wait_with_inst_blob`, `submit_wait_failed`, `bo_roundtrip_ok`,
    /// `bo_roundtrip_stub_ok`, …
    pub status: String,
    /// Human-readable detail (error text or notes).
    pub detail: String,
}

impl TimedOpResult {
    /// Serialize as a single JSON object (serde_json) for CLI / colibrì tools.
    ///
    /// Always includes [`crate::TIMED_OP_JSON_SCHEMA_VERSION`] as `schema_version`.
    /// See `docs/COLIBRI_ADAPTER.md`.
    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "schema_version": crate::TIMED_OP_JSON_SCHEMA_VERSION,
            "op": self.op,
            "m": self.m,
            "n": self.n,
            "k": self.k,
            "size_bytes": self.size_bytes,
            "dtype": self.dtype,
            "total_s": self.total_s,
            "host_prep_s": self.host_prep_s,
            "hwctx_s": self.hwctx_s,
            "fill_s": self.fill_s,
            "submit_wait_s": self.submit_wait_s,
            "device_s": self.device_s,
            "host_ref_s": self.host_ref_s,
            "host_ref_ok": self.host_ref_ok,
            "host_ref_detail": self.host_ref_detail,
            "ran_on_npu": self.ran_on_npu,
            "ran_on_device_path": self.ran_on_device_path,
            "real_ioctl": self.real_ioctl,
            "status": self.status,
            "detail": self.detail,
            "efficiency_notes": self.efficiency_notes(),
        })
    }

    /// Human-readable stage efficiency summary (Phase 6 profiling).
    ///
    /// Compares host prep / fill / device(sync) / submit_wait against wall
    /// `total_s`. **Does not** claim silicon GEMM tops or NPU compute Pass.
    /// For `bo_roundtrip`, emphasizes DMA-only metrics (`device_s` = SYNC FROM).
    #[must_use]
    pub fn efficiency_notes(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        parts.push(format!(
            "op={} status={} total_s={:.6}",
            self.op, self.status, self.total_s
        ));
        parts.push(format!("host_prep_s={:.6}", self.host_prep_s));
        if let Some(f) = self.fill_s {
            parts.push(format!("fill_s={f:.6}"));
        }
        if let Some(d) = self.device_s {
            if self.op == "bo_roundtrip" {
                // Only claim driver DMA when real ioctl path actually ran (Issue 5).
                if self.real_ioctl && self.ran_on_device_path {
                    parts.push(format!("device_sync_from_s={d:.6} (DMA-only; not GEMM)"));
                } else {
                    parts.push(format!(
                        "device_s={d:.6} (stub; not driver DMA; ran_on_device_path={})",
                        self.ran_on_device_path
                    ));
                }
            } else {
                parts.push(format!("device_s={d:.6}"));
            }
        }
        if let Some(s) = self.submit_wait_s {
            parts.push(format!("submit_wait_s={s:.6}"));
        }
        if let Some(h) = self.hwctx_s {
            parts.push(format!("hwctx_s={h:.6}"));
        }
        if let Some(hr) = self.host_ref_s {
            parts.push(format!("host_ref_s={hr:.6} (CPU golden only)"));
        }
        // Ratio notes when denominators are positive.
        if self.total_s > 0.0 {
            let host_frac = self.host_prep_s / self.total_s;
            parts.push(format!("host_prep/total={host_frac:.3}"));
            if let Some(d) = self.device_s {
                let dev_frac = d / self.total_s;
                parts.push(format!("device/total={dev_frac:.3}"));
            }
        }
        parts.push(format!(
            "ran_on_npu={} ran_on_device_path={} real_ioctl={}",
            self.ran_on_npu, self.ran_on_device_path, self.real_ioctl
        ));
        if self.ran_on_npu {
            // Should stay false until Phase 3.5 numerical verify; still honest.
            parts.push(
                "WARNING: ran_on_npu set — verify numerical GEMM before claiming tops".to_string(),
            );
        } else {
            parts.push(
                "honesty: efficiency stages ≠ silicon GEMM tops (txn still opaque)".to_string(),
            );
        }
        if let Some(sz) = self.size_bytes {
            parts.push(format!("size_bytes={sz}"));
            if self.op == "bo_roundtrip"
                && let Some(d) = self.device_s
                && d > 0.0
                && sz > 0
            {
                #[allow(clippy::cast_precision_loss)]
                let mib_s = (sz as f64) / d / (1024.0 * 1024.0);
                parts.push(format!(
                    "approx_sync_from_MiB_s={mib_s:.3} (host clock; not NPU compute)"
                ));
            }
        }
        parts.join("; ")
    }
}

/// Classify `submit_and_wait` failures into stable machine statuses.
///
/// Does **not** claim every failure is a payload problem — reserve
/// `payload_unsupported` for messages that look payload/txn related.
fn classify_submit_wait_error(err: &DirectError) -> &'static str {
    let s = err.to_string().to_ascii_lowercase();
    if s.contains("timeout") || s.contains("etimedout") || s.contains("etime") {
        "wait_timeout"
    } else if s.contains("eperm") || s.contains("permission denied") || s.contains("exec_cmd_allow")
    {
        "exec_cmd_perm"
    } else if s.contains("payload")
        || s.contains("txn")
        || s.contains("not implemented")
        || s.contains("invalid argument")
        || s.contains("einval")
    {
        "payload_unsupported"
    } else {
        "submit_wait_failed"
    }
}

/// Host-side f32 matmul reference result (never NPU).
#[derive(Debug, Clone)]
pub struct HostMatmulRef {
    /// Wall time for the pure host multiply, seconds.
    pub elapsed_s: f64,
    /// First output element C[0,0] (useful for shape sanity).
    pub c00: f32,
    /// Simple FNV-1a-ish checksum over all output bits (stable for JSON).
    pub checksum: u64,
    /// Number of output elements (`m * n`).
    pub out_elems: u64,
}

/// Decode hex string (optional `0x` prefix, whitespace ignored) into bytes.
///
/// Refuses payloads larger than [`MAX_INSTRUCTION_BLOB_BYTES`].
///
/// # Errors
///
/// [`DirectError::InvalidArgument`] on odd length, non-hex characters, or oversize.
pub fn decode_instruction_hex(hex: &str) -> Result<Vec<u8>, DirectError> {
    let mut s: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
    if let Some(rest) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        s = rest.to_string();
    }
    if s.is_empty() {
        return Err(DirectError::InvalidArgument(
            "instruction hex is empty after strip".to_string(),
        ));
    }
    if !s.len().is_multiple_of(2) {
        return Err(DirectError::InvalidArgument(format!(
            "instruction hex length {} is odd",
            s.len()
        )));
    }
    let nbytes = (s.len() / 2) as u64;
    if nbytes > MAX_INSTRUCTION_BLOB_BYTES {
        return Err(DirectError::InvalidArgument(format!(
            "instruction hex decodes to {nbytes} bytes > max {MAX_INSTRUCTION_BLOB_BYTES}"
        )));
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        let hi = (bytes[idx] as char).to_digit(16).ok_or_else(|| {
            DirectError::InvalidArgument(format!(
                "invalid hex char {:?} at index {idx}",
                bytes[idx] as char
            ))
        })?;
        let lo = (bytes[idx + 1] as char).to_digit(16).ok_or_else(|| {
            DirectError::InvalidArgument(format!(
                "invalid hex char {:?} at index {}",
                bytes[idx + 1] as char,
                idx + 1
            ))
        })?;
        #[allow(clippy::cast_possible_truncation)]
        let byte = ((hi << 4) | lo) as u8;
        out.push(byte);
        idx += 2;
    }
    Ok(out)
}

/// Load instruction bytes from a filesystem path (no proprietary defaults).
///
/// Refuses files larger than [`MAX_INSTRUCTION_BLOB_BYTES`] (checked via metadata
/// before full read when possible).
///
/// # Errors
///
/// Missing/empty/oversize file or I/O failure.
pub fn load_instruction_blob_from_path(path: &str) -> Result<Vec<u8>, DirectError> {
    let path = path.trim();
    if path.is_empty() {
        return Err(DirectError::InvalidArgument(
            "instruction blob path is empty".to_string(),
        ));
    }
    let meta = std::fs::metadata(path).map_err(|e| {
        DirectError::InvalidArgument(format!("instruction blob metadata failed ({path}): {e}"))
    })?;
    let len = meta.len();
    if len == 0 {
        return Err(DirectError::InvalidArgument(format!(
            "instruction blob file is empty: {path}"
        )));
    }
    if len > MAX_INSTRUCTION_BLOB_BYTES {
        return Err(DirectError::InvalidArgument(format!(
            "instruction blob file is {len} bytes > max {MAX_INSTRUCTION_BLOB_BYTES}: {path}"
        )));
    }
    let bytes = std::fs::read(path).map_err(|e| {
        DirectError::InvalidArgument(format!("instruction blob read failed ({path}): {e}"))
    })?;
    if bytes.is_empty() {
        return Err(DirectError::InvalidArgument(format!(
            "instruction blob file is empty: {path}"
        )));
    }
    if (bytes.len() as u64) > MAX_INSTRUCTION_BLOB_BYTES {
        return Err(DirectError::InvalidArgument(format!(
            "instruction blob read size {} > max {MAX_INSTRUCTION_BLOB_BYTES}: {path}",
            bytes.len()
        )));
    }
    Ok(bytes)
}

/// Load optional instruction blob from env (no proprietary data hardcoded).
///
/// Priority:
/// 1. `XDNA2_INSTRUCTION_BLOB=/path/to/file` — raw bytes from file
/// 2. `XDNA2_INSTRUCTION_HEX=...` — hex string (optional `0x`, whitespace ok)
///
/// Returns `Ok(None)` when neither is set. Empty file / empty hex → error.
///
/// # Errors
///
/// I/O or decode failures when the env var is set but content is unusable.
pub fn load_instruction_blob_from_env() -> Result<Option<Vec<u8>>, DirectError> {
    if let Ok(path) = std::env::var("XDNA2_INSTRUCTION_BLOB") {
        return Ok(Some(load_instruction_blob_from_path(&path)?));
    }
    if let Ok(hex) = std::env::var("XDNA2_INSTRUCTION_HEX") {
        let hex = hex.trim();
        if hex.is_empty() {
            return Err(DirectError::InvalidArgument(
                "XDNA2_INSTRUCTION_HEX is set but empty".to_string(),
            ));
        }
        return Ok(Some(decode_instruction_hex(hex)?));
    }
    Ok(None)
}

/// Pure host f32 matmul `C = A * B` with deterministic fill (never NPU).
///
/// `A` is `m×k` with `A[i,j] = 1.0`; `B` is `k×n` with `B[j,l] = 1.0`;
/// thus `C[i,l] = k as f32`. Useful as a shape/checksum baseline when NPU
/// GEMM is blocked.
///
/// # Errors
///
/// [`DirectError::InvalidArgument`] when dimensions are zero or product exceeds
/// [`HOST_REF_MAX_ELEMENTS`].
#[allow(clippy::many_single_char_names, clippy::cast_precision_loss)]
pub fn host_matmul_f32_ones(m: u32, n: u32, k: u32) -> Result<HostMatmulRef, DirectError> {
    use std::time::Instant;

    if m == 0 || n == 0 || k == 0 {
        return Err(DirectError::InvalidArgument(
            "host_matmul_f32_ones requires m,n,k > 0".to_string(),
        ));
    }
    let m_u = u64::from(m);
    let n_u = u64::from(n);
    let k_u = u64::from(k);
    let elems = m_u
        .checked_mul(n_u)
        .and_then(|x| x.checked_mul(k_u))
        .ok_or_else(|| {
            DirectError::InvalidArgument("host_matmul element product overflow".to_string())
        })?;
    if elems > HOST_REF_MAX_ELEMENTS {
        return Err(DirectError::InvalidArgument(format!(
            "host_matmul refuses m*n*k={elems} > max {HOST_REF_MAX_ELEMENTS}"
        )));
    }
    let out_elems = m_u.saturating_mul(n_u);
    let out_len = usize::try_from(out_elems).map_err(|_| {
        DirectError::InvalidArgument("host_matmul output length exceeds usize".to_string())
    })?;
    let t0 = Instant::now();
    let mut c = vec![0.0_f32; out_len];
    let k_f = k as f32;
    let rows = m as usize;
    let cols = n as usize;
    let inner = k as usize;
    // A,B all-ones → C all k_f; nested loop so host_ref_s is a real timing.
    for row in 0..rows {
        for col in 0..cols {
            let mut acc = 0.0_f32;
            for _kk in 0..inner {
                acc = f32::mul_add(1.0, 1.0, acc);
            }
            c[row * cols + col] = acc;
        }
    }
    let elapsed_s = t0.elapsed().as_secs_f64();
    let c00 = c.first().copied().unwrap_or(0.0);
    if (c00 - k_f).abs() > 1e-3 {
        return Err(DirectError::InvalidArgument(format!(
            "host_matmul internal check failed: c00={c00} expected {k_f}"
        )));
    }
    let mut checksum: u64 = 0xcbf2_9ce4_8422_2325;
    for v in &c {
        let bits = u64::from(v.to_bits());
        checksum ^= bits;
        checksum = checksum.wrapping_mul(0x0100_0000_01b3);
    }
    Ok(HostMatmulRef {
        elapsed_s,
        c00,
        checksum,
        out_elems,
    })
}

/// Whether to attach host golden matmul (env `XDNA2_HOST_REF=1` or small f32 shapes).
fn want_host_ref_matmul(m: u32, n: u32, k: u32, dtype: &str) -> bool {
    if !dtype.eq_ignore_ascii_case("f32") {
        return false;
    }
    if std::env::var("XDNA2_HOST_REF").is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true")) {
        return true;
    }
    // Auto for modest shapes so colibrì gets a baseline without env.
    let elems = u64::from(m)
        .saturating_mul(u64::from(n))
        .saturating_mul(u64::from(k));
    elems > 0 && elems <= 256 * 1024
}

fn attach_host_ref(result: &mut TimedOpResult, m: u32, n: u32, k: u32, dtype: &str) {
    if !want_host_ref_matmul(m, n, k, dtype) {
        result.host_ref_ok = false;
        result.host_ref_s = None;
        if result.host_ref_detail.is_empty() {
            result.host_ref_detail =
                "host_ref skipped (set XDNA2_HOST_REF=1 for f32, or use smaller shape)".to_string();
        }
        return;
    }
    match host_matmul_f32_ones(m, n, k) {
        Ok(r) => {
            result.host_ref_ok = true;
            result.host_ref_s = Some(r.elapsed_s);
            result.host_ref_detail = format!(
                "host_ref_f32 ones-matmul c00={} checksum={:#x} out_elems={} (CPU only, not NPU)",
                r.c00, r.checksum, r.out_elems
            );
        }
        Err(e) => {
            result.host_ref_ok = false;
            result.host_ref_s = None;
            result.host_ref_detail = format!("host_ref failed: {e}");
        }
    }
}

fn timed_op_base(
    op: &str,
    m: u32,
    n: u32,
    k: u32,
    dtype: &str,
    real_ioctl: bool,
    size_bytes: Option<u64>,
) -> TimedOpResult {
    TimedOpResult {
        op: op.to_string(),
        m,
        n,
        k,
        size_bytes,
        dtype: dtype.to_string(),
        total_s: 0.0,
        host_prep_s: 0.0,
        hwctx_s: None,
        fill_s: None,
        submit_wait_s: None,
        device_s: None,
        host_ref_s: None,
        host_ref_ok: false,
        host_ref_detail: String::new(),
        ran_on_npu: false,
        ran_on_device_path: false,
        real_ioctl,
        status: String::new(),
        detail: String::new(),
    }
}

/// Destroy a hwctx and GEM_CLOSE any non-zero UMQ/log BOs it owns.
///
/// Prefer this over bare [`DirectSubmission::destroy_hwctx`] when the context
/// was created with Strategy-B Shmem UMQ/log (non-zero handles). KMQ zero-BO
/// contexts (`umq_bo=0`, `log_buf_bo=0`) only destroy the hwctx.
#[inline]
pub fn destroy_hwctx_and_owned_bos(submission: &dyn DirectSubmission, ctx: &HardwareContext) {
    let _ = submission.destroy_hwctx(0, ctx.handle);
    if ctx.umq_bo != 0 {
        let _ = submission.destroy_buffer(0, ctx.umq_bo);
    }
    if ctx.log_buf_bo != 0 {
        let _ = submission.destroy_buffer(0, ctx.log_buf_bo);
    }
}

/// Attempt a minimal timed matmul (or honest structured failure).
///
/// **Phase 3 / 3.5 honesty:** CREATE_HWCTX and/or firmware txn payload still block
/// real GEMM on silicon by default. This function:
/// 1. Times `create_hwctx` (with valid qos_p + DevHeap path).
/// 2. On success, if [`exec_cmd_is_enabled`]: optional instruction blob
///    (`XDNA2_INSTRUCTION_BLOB` / `XDNA2_INSTRUCTION_HEX`) → device BO + START_NPU
///    with `xdna_addr`; else empty-inst envelope. Always **host-fills** Cmd BO
///    then [`submit_and_wait_command`]. Statuses:
///    - `ok_submit_wait_envelope` — accept+wait, no instruction blob
///    - `ok_submit_wait_with_inst_blob` — accept+wait with non-zero inst addr
///    - **`ran_on_npu=false`** unless `XDNA2_LAB_OFM=1` + OFM readback +
///      [`super::lab_io::lab_decide_from_ofm_bytes`] (all decide criteria)
///    - `ok_npu_verified` only when lab OFM decide returns true
/// 3. If L1 closed: `hwctx_ok`, no EXEC_CMD.
/// 4. Optional host golden matmul (`host_ref_*`) when blocked / for baseline.
///
/// `m,n,k` are recorded for gate tables even when the op does not run on NPU.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // m,n,k conventional
pub fn timed_matmul_direct(
    submission: &dyn DirectSubmission,
    m: u32,
    n: u32,
    k: u32,
    dtype: &str,
) -> TimedOpResult {
    use std::time::Instant;

    let t0 = Instant::now();
    let real_ioctl = submission.is_real_ioctl_path();
    let mut host_prep_s = 0.0_f64;
    let mut fill_s: Option<f64> = None;
    let mut submit_wait_s: Option<f64> = None;

    // Env wins; else Phase 6 heuristic from resource TOPS when GET works (else 4).
    let default_tiles = submission
        .get_resource_info()
        .map_or(DEFAULT_HWCTX_TILES, |r| suggest_hwctx_tiles(&r));
    let tiles = parse_u32_from_env("XDNA2_HWCTX_TILES_SINGLE", default_tiles).max(1);
    let mem = parse_u32_from_env("XDNA2_HWCTX_MEM_SINGLE", 256 * 1024);
    let umq = parse_u64_from_env("XDNA2_HWCTX_UMQ", 16 * 1024).max(4096);
    let log = parse_u64_from_env("XDNA2_HWCTX_LOG", 128 * 1024).max(4096);

    let prep0 = Instant::now();
    let qos = QosInfo::default();
    let hwctx = submission.create_hwctx_with_sizes(0, tiles, mem, umq, log, Some(&qos));
    let hwctx_elapsed = prep0.elapsed().as_secs_f64();
    let hwctx_s = Some(hwctx_elapsed);
    host_prep_s += hwctx_elapsed;

    let ctx = match hwctx {
        Ok(c) => c,
        Err(e) => {
            let mut r = timed_op_base("matmul", m, n, k, dtype, real_ioctl, None);
            r.total_s = t0.elapsed().as_secs_f64();
            r.host_prep_s = host_prep_s;
            r.hwctx_s = hwctx_s;
            r.status = "hwctx_blocked".to_string();
            r.detail = format!(
                "CREATE_HWCTX failed (direct path): {e}. \
                 Real GEMM requires a working hwctx + known-good instruction blob + verify."
            );
            attach_host_ref(&mut r, m, n, k, dtype);
            return r;
        }
    };

    // CRITICAL SAFETY: see exec_cmd_is_enabled() / docs/XDNA2_NEVER_AGAIN.md.
    let try_submit = exec_cmd_is_enabled();

    let (status, detail, ran_on_npu) = if try_submit {
        let fill0 = Instant::now();
        let timeout_ms =
            parse_u32_from_env("XDNA2_WAIT_TIMEOUT_MS", DEFAULT_WAIT_CMD_TIMEOUT_MS).max(1);

        // Optional instruction blob → device BO → xdna_addr for START_NPU.buffer.
        let mut inst_handle: Option<u32> = None;
        let mut used_inst_blob = false;
        let mut inst_addr: u64 = 0;
        let mut inst_size: u32 = 0;
        let mut blob_note = String::from("envelope (no instruction blob)");

        match load_instruction_blob_from_env() {
            Ok(None) => {}
            Ok(Some(blob)) => {
                let blob_len = blob.len() as u64;
                match submission.create_buffer(0, blob_len.max(4096), BufferType::Shmem) {
                    Ok(bh) => {
                        if let Err(e) = submission.write_bo_bytes(0, bh.0, 0, &blob) {
                            let _ = submission.destroy_buffer(0, bh.0);
                            fill_s = Some(fill0.elapsed().as_secs_f64());
                            host_prep_s += fill_s.unwrap_or(0.0);
                            destroy_hwctx_and_owned_bos(submission, &ctx);
                            let mut r = timed_op_base("matmul", m, n, k, dtype, real_ioctl, None);
                            r.total_s = t0.elapsed().as_secs_f64();
                            r.host_prep_s = host_prep_s;
                            r.hwctx_s = hwctx_s;
                            r.fill_s = fill_s;
                            r.status = "inst_blob_fill_failed".to_string();
                            r.detail = format!(
                                "instruction blob loaded ({} bytes) but write_bo_bytes failed: {e}",
                                blob.len()
                            );
                            attach_host_ref(&mut r, m, n, k, dtype);
                            return r;
                        }
                        match submission.get_bo_info(0, bh.0) {
                            Ok(info) if info.xdna_addr != 0 => {
                                inst_addr = info.xdna_addr;
                                inst_size = u32::try_from(blob.len()).unwrap_or(u32::MAX);
                                inst_handle = Some(bh.0);
                                used_inst_blob = true;
                                blob_note = format!(
                                    "inst_blob {} bytes xdna_addr={:#x}",
                                    blob.len(),
                                    inst_addr
                                );
                            }
                            Ok(info) => {
                                let _ = submission.destroy_buffer(0, bh.0);
                                fill_s = Some(fill0.elapsed().as_secs_f64());
                                host_prep_s += fill_s.unwrap_or(0.0);
                                destroy_hwctx_and_owned_bos(submission, &ctx);
                                let mut r =
                                    timed_op_base("matmul", m, n, k, dtype, real_ioctl, None);
                                r.total_s = t0.elapsed().as_secs_f64();
                                r.host_prep_s = host_prep_s;
                                r.hwctx_s = hwctx_s;
                                r.fill_s = fill_s;
                                r.status = "inst_blob_addr_zero".to_string();
                                r.detail = format!(
                                    "instruction BO created but xdna_addr=0 (map_offset={:#x}); \
                                     refusing START_NPU with zero device addr",
                                    info.map_offset
                                );
                                attach_host_ref(&mut r, m, n, k, dtype);
                                return r;
                            }
                            Err(e) => {
                                let _ = submission.destroy_buffer(0, bh.0);
                                fill_s = Some(fill0.elapsed().as_secs_f64());
                                host_prep_s += fill_s.unwrap_or(0.0);
                                destroy_hwctx_and_owned_bos(submission, &ctx);
                                let mut r =
                                    timed_op_base("matmul", m, n, k, dtype, real_ioctl, None);
                                r.total_s = t0.elapsed().as_secs_f64();
                                r.host_prep_s = host_prep_s;
                                r.hwctx_s = hwctx_s;
                                r.fill_s = fill_s;
                                r.status = "inst_blob_bo_info_failed".to_string();
                                r.detail = format!("get_bo_info for instruction BO failed: {e}");
                                attach_host_ref(&mut r, m, n, k, dtype);
                                return r;
                            }
                        }
                    }
                    Err(e) => {
                        fill_s = Some(fill0.elapsed().as_secs_f64());
                        host_prep_s += fill_s.unwrap_or(0.0);
                        destroy_hwctx_and_owned_bos(submission, &ctx);
                        let mut r = timed_op_base("matmul", m, n, k, dtype, real_ioctl, None);
                        r.total_s = t0.elapsed().as_secs_f64();
                        r.host_prep_s = host_prep_s;
                        r.hwctx_s = hwctx_s;
                        r.fill_s = fill_s;
                        r.status = "inst_blob_bo_failed".to_string();
                        r.detail = format!("CREATE_BO for instruction blob failed: {e}");
                        attach_host_ref(&mut r, m, n, k, dtype);
                        return r;
                    }
                }
            }
            Err(e) => {
                fill_s = Some(fill0.elapsed().as_secs_f64());
                host_prep_s += fill_s.unwrap_or(0.0);
                destroy_hwctx_and_owned_bos(submission, &ctx);
                let mut r = timed_op_base("matmul", m, n, k, dtype, real_ioctl, None);
                r.total_s = t0.elapsed().as_secs_f64();
                r.host_prep_s = host_prep_s;
                r.hwctx_s = hwctx_s;
                r.fill_s = fill_s;
                r.status = "inst_blob_load_failed".to_string();
                r.detail = format!("instruction blob env present but load failed: {e}");
                attach_host_ref(&mut r, m, n, k, dtype);
                return r;
            }
        }

        // Optional lab OFM path (`XDNA2_LAB_OFM=1`): IFM/WTS/OFM Shmem BOs +
        // PARTIAL_ELF kernel args with prop_count=0. Default off → empty args,
        // hard-false ran_on_npu (historical path).
        //
        // Optional model WTS (`XDNA2_LAB_WEIGHT=1` + `XDNA2_LAB_WEIGHT_MODEL`):
        // requires LAB_OFM; fail-closed if model/select/dims fail. Golden uses
        // ones IFM × model WTS (not ones×ones). Caller must still have known-good
        // txn for product ran_on_npu.
        //
        // Always pin instruction BO in EXEC arg list when present (XRT also
        // binds ctrl BO), whether or not LAB_OFM I/O BOs exist.
        // WeightExecBind is not RAII — destroy_weight_exec_bind / destroy_io_bos
        // on every exit path below.
        let lab_ofm = super::lab_io::lab_ofm_env_enabled();
        let lab_weight = super::lab_io::lab_weight_env_enabled();
        let mut io_set: Option<super::lab_io::IoBoSet> = None;
        // When set, OFM decide uses model WTS golden (ones IFM × model WTS).
        // WeightExecBind is not RAII — BO ownership lives in `io_set` only.
        let mut ofm_host_golden: Option<Vec<f32>> = None;
        let mut kernel_args: Vec<u32> = Vec::new();
        let mut arg_bo_handles = super::lab_io::build_lab_arg_bo_handles(None, inst_handle);
        let mut ofm_note = String::new();
        if lab_ofm {
            let setup: Result<super::lab_io::IoBoSet, String> = if lab_weight {
                match super::host_graph::create_lab_weight_io_from_env(submission, m, n, k, dtype) {
                    Ok((bind, golden)) => {
                        ofm_note = format!(
                            "lab_ofm+lab_weight model WTS bound name={} ofm_bytes={}",
                            bind.wts_name, bind.ofm_payload_nbytes
                        );
                        ofm_host_golden = Some(golden);
                        // Move BOs into io_set; drop metadata shell (no second destroy).
                        Ok(bind.io)
                    }
                    Err(e) => Err(format!("XDNA2_LAB_WEIGHT setup failed: {e}")),
                }
            } else {
                super::lab_io::create_and_fill_ones_io_bos(submission, m, n, k, dtype)
                    .map_err(|e| format!("ones IFM/WTS/OFM: {e}"))
            };
            match setup {
                Ok(ios) => {
                    kernel_args = super::lab_io::pack_start_npu_kernel_args(
                        ios.ifm_addr,
                        ios.wts_addr,
                        ios.ofm_addr,
                    );
                    // IFM/WTS/OFM first, then instruction (if any) for pin list.
                    arg_bo_handles = super::lab_io::build_lab_arg_bo_handles(
                        Some((ios.ifm.0, ios.wts.0, ios.ofm.0)),
                        inst_handle,
                    );
                    if ofm_note.is_empty() {
                        ofm_note = format!(
                            "lab_ofm IFM/WTS/OFM bound (prop_count=0 kernel_args={} ofm_bytes={})",
                            kernel_args.len(),
                            ios.ofm_bytes
                        );
                    } else {
                        ofm_note = format!(
                            "{ofm_note}; kernel_args={} ofm_bytes={}",
                            kernel_args.len(),
                            ios.ofm_bytes
                        );
                    }
                    io_set = Some(ios);
                }
                Err(e) => {
                    // Fail closed: do not degrade to envelope EXEC when operator
                    // requested LAB_OFM (avoids false "ok_submit_wait_*" without OFM).
                    if let Some(h) = inst_handle {
                        let _ = submission.destroy_buffer(0, h);
                    }
                    fill_s = Some(fill0.elapsed().as_secs_f64());
                    host_prep_s += fill_s.unwrap_or(0.0);
                    destroy_hwctx_and_owned_bos(submission, &ctx);
                    let mut r = timed_op_base("matmul", m, n, k, dtype, real_ioctl, None);
                    r.total_s = t0.elapsed().as_secs_f64();
                    r.host_prep_s = host_prep_s;
                    r.hwctx_s = hwctx_s;
                    r.fill_s = fill_s;
                    r.status = super::lab_io::classify_lab_matmul_status(
                        true,  // lab_ofm
                        false, // setup failed
                        false, false, false, false, false, false,
                    )
                    .to_string();
                    r.detail = format!(
                        "XDNA2_LAB_OFM=1 but IFM/WTS/OFM create/fill/zero failed; \
                         refusing EXEC (fail-closed): {e}"
                    );
                    r.ran_on_npu = false;
                    attach_host_ref(&mut r, m, n, k, dtype);
                    return r;
                }
            }
        } else if let Err(e) = super::host_graph::lab_weight_requires_ofm(lab_ofm, lab_weight) {
            // LAB_WEIGHT without LAB_OFM is invalid — fail closed (no silent ignore).
            if let Some(h) = inst_handle {
                let _ = submission.destroy_buffer(0, h);
            }
            fill_s = Some(fill0.elapsed().as_secs_f64());
            host_prep_s += fill_s.unwrap_or(0.0);
            destroy_hwctx_and_owned_bos(submission, &ctx);
            let mut r = timed_op_base("matmul", m, n, k, dtype, real_ioctl, None);
            r.total_s = t0.elapsed().as_secs_f64();
            r.host_prep_s = host_prep_s;
            r.hwctx_s = hwctx_s;
            r.fill_s = fill_s;
            r.status = "lab_ofm_setup_failed".to_string();
            r.detail = e.to_string();
            r.ran_on_npu = false;
            attach_host_ref(&mut r, m, n, k, dtype);
            return r;
        }

        let payload = if kernel_args.is_empty() {
            build_ert_start_npu_cmd_bo(inst_addr, inst_size, &[], 0x1)
        } else {
            build_ert_start_npu_cmd_bo_with_kernel_args(inst_addr, inst_size, &kernel_args, 0x1)
        };
        let payload = match payload {
            Ok(p) => p,
            Err(e) => {
                if let Some(ref ios) = io_set {
                    super::lab_io::destroy_io_bos(submission, ios);
                }
                if let Some(h) = inst_handle {
                    let _ = submission.destroy_buffer(0, h);
                }
                fill_s = Some(fill0.elapsed().as_secs_f64());
                host_prep_s += fill_s.unwrap_or(0.0);
                destroy_hwctx_and_owned_bos(submission, &ctx);
                let mut r = timed_op_base("matmul", m, n, k, dtype, real_ioctl, None);
                r.total_s = t0.elapsed().as_secs_f64();
                r.host_prep_s = host_prep_s;
                r.hwctx_s = hwctx_s;
                r.fill_s = fill_s;
                r.status = "payload_build_failed".to_string();
                r.detail = format!("build_ert_start_npu_cmd_bo failed: {e}");
                attach_host_ref(&mut r, m, n, k, dtype);
                return r;
            }
        };

        let cmd_bo = submission.create_buffer(0, payload.len() as u64, BufferType::Cmd);
        match cmd_bo {
            Ok(handle) => {
                if let Err(e) = fill_cmd_bo_with_payload(submission, 0, handle.0, &payload) {
                    let _ = submission.destroy_buffer(0, handle.0);
                    if let Some(ref ios) = io_set {
                        super::lab_io::destroy_io_bos(submission, ios);
                    }
                    if let Some(h) = inst_handle {
                        let _ = submission.destroy_buffer(0, h);
                    }
                    fill_s = Some(fill0.elapsed().as_secs_f64());
                    host_prep_s += fill_s.unwrap_or(0.0);
                    (
                        "cmd_bo_fill_failed".to_string(),
                        format!(
                            "refusing EXEC_CMD: host fill of Cmd BO failed (handle={}): {e}. \
                             Empty Cmd BO is the 2026-07-13 wedge class.",
                            handle.0
                        ),
                        false,
                    )
                } else {
                    fill_s = Some(fill0.elapsed().as_secs_f64());
                    host_prep_s += fill_s.unwrap_or(0.0);
                    let cmd = Command {
                        ty: CommandType::SubmitExecBuf,
                        hwctx_handle: ctx.handle,
                        payload,
                        cmd_bo_handle: Some(handle.0),
                        arg_bo_handles,
                    };
                    let d0 = Instant::now();
                    let sw = submit_and_wait_command(submission, 0, &cmd, timeout_ms);
                    let ds = d0.elapsed().as_secs_f64();
                    submit_wait_s = Some(ds);
                    let _ = submission.destroy_buffer(0, handle.0);
                    match sw {
                        Ok(r) => {
                            // Default: never claim NPU GEMM from envelope/blob alone.
                            let mut ran = false;
                            let mut ofm_readback_ok = false;
                            let mut ofm_allclose_failed = false;
                            let mut ofm_decide_error = false;
                            let mut detail = format!(
                                "EXEC_CMD+WAIT accepted seq={} timeout_ms={}; {blob_note}; \
                                 ran_on_npu=false (no numerical verify). Destroying hwctx.",
                                r.seq, r.timeout_ms
                            );
                            if lab_ofm {
                                if let Some(ref ios) = io_set {
                                    match super::lab_io::readback_ofm(submission, ios) {
                                        Ok(ofm_bytes) => {
                                            ofm_readback_ok = true;
                                            let plan = super::lab_io::LabOfmVerifyPlan {
                                                m,
                                                n,
                                                k,
                                                used_inst_blob,
                                                submit_wait_ok: true,
                                                real_ioctl,
                                                ofm_bytes,
                                                criteria:
                                                    super::numerical::NumericalVerifyCriteria::default(
                                                    ),
                                            };
                                            let decide = ofm_host_golden.as_ref().map_or_else(
                                                || super::lab_io::lab_decide_from_ofm_bytes(&plan),
                                                |g| {
                                                    super::lab_io::lab_decide_from_ofm_bytes_with_golden(
                                                        &plan, g,
                                                    )
                                                },
                                            );
                                            match decide {
                                                Ok((true, _)) => {
                                                    ran = true;
                                                    detail = format!(
                                                        "EXEC_CMD+WAIT seq={} + OFM numerical verify \
                                                         PASS (lab_ofm{}); {blob_note}; {ofm_note}",
                                                        r.seq,
                                                        if ofm_host_golden.is_some() {
                                                            "+model_wts"
                                                        } else {
                                                            ""
                                                        }
                                                    );
                                                }
                                                Ok((false, reason)) => {
                                                    detail = format!(
                                                        "EXEC_CMD+WAIT seq={} accepted but OFM verify \
                                                         refused ({reason:?}); {blob_note}; {ofm_note}; \
                                                         ran_on_npu=false",
                                                        r.seq
                                                    );
                                                    if reason
                                                        == Some(
                                                            super::numerical::RanOnNpuBlockReason::AllcloseFailed,
                                                        )
                                                    {
                                                        ofm_allclose_failed = true;
                                                    }
                                                }
                                                Err(e) => {
                                                    // Readback succeeded; decide path failed (golden/decode).
                                                    ofm_decide_error = true;
                                                    detail = format!(
                                                        "EXEC_CMD+WAIT ok but OFM decide error: {e}; \
                                                         {blob_note}; {ofm_note}"
                                                    );
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            detail = format!(
                                                "EXEC_CMD+WAIT ok but OFM readback failed: {e}; \
                                                 {blob_note}; {ofm_note}"
                                            );
                                        }
                                    }
                                } else if !ofm_note.is_empty() {
                                    detail = format!("{detail} {ofm_note}");
                                }
                            }
                            let st = super::lab_io::classify_lab_matmul_status(
                                lab_ofm,
                                io_set.is_some() || !lab_ofm,
                                true, // submit_wait_ok
                                used_inst_blob,
                                ofm_readback_ok,
                                ran,
                                ofm_allclose_failed,
                                ofm_decide_error,
                            )
                            .to_string();
                            if let Some(ref ios) = io_set {
                                super::lab_io::destroy_io_bos(submission, ios);
                            }
                            if let Some(h) = inst_handle {
                                let _ = submission.destroy_buffer(0, h);
                            }
                            (st, detail, ran)
                        }
                        Err(e) => {
                            if let Some(ref ios) = io_set {
                                super::lab_io::destroy_io_bos(submission, ios);
                            }
                            if let Some(h) = inst_handle {
                                let _ = submission.destroy_buffer(0, h);
                            }
                            let st = classify_submit_wait_error(&e);
                            (
                                st.to_string(),
                                format!(
                                    "hwctx ok (handle={}) Cmd BO host-filled ({blob_note}) but \
                                     submit_and_wait failed ({st}): {e}. {ofm_note}",
                                    ctx.handle
                                ),
                                false,
                            )
                        }
                    }
                }
            }
            Err(e) => {
                if let Some(ref ios) = io_set {
                    super::lab_io::destroy_io_bos(submission, ios);
                }
                if let Some(h) = inst_handle {
                    let _ = submission.destroy_buffer(0, h);
                }
                fill_s = Some(fill0.elapsed().as_secs_f64());
                host_prep_s += fill_s.unwrap_or(0.0);
                (
                    "cmd_bo_failed".to_string(),
                    format!("CREATE_BO(Cmd) failed after hwctx success: {e}"),
                    false,
                )
            }
        }
    } else {
        (
            "hwctx_ok".to_string(),
            format!(
                "CREATE_HWCTX succeeded (handle={}); EXEC_CMD not attempted ({}). \
                 Destroying hwctx. Prefer timed_bo_roundtrip for safe silicon timings.",
                ctx.handle,
                exec_cmd_block_reason()
            ),
            false,
        )
    };

    destroy_hwctx_and_owned_bos(submission, &ctx);

    let mut result = timed_op_base("matmul", m, n, k, dtype, real_ioctl, None);
    result.total_s = t0.elapsed().as_secs_f64();
    result.host_prep_s = host_prep_s;
    result.hwctx_s = hwctx_s;
    result.fill_s = fill_s;
    result.submit_wait_s = submit_wait_s;
    result.device_s = submit_wait_s;
    // Phase 3.5: ran_on_npu true **only** via lab OFM readback + decide_ran_on_npu.
    // Envelope / inst-blob submit alone never sets true (default path hard-false).
    result.ran_on_npu = ran_on_npu;
    result.ran_on_device_path = false;
    result.status = status;
    result.detail = detail;
    attach_host_ref(&mut result, m, n, k, dtype);
    result
}

/// Timed Shmem BO create → fill pattern → SYNC TO_DEVICE → SYNC FROM_DEVICE → verify.
///
/// **Always safe under default gates** (no EXEC_CMD).
///
/// **Honesty:**
/// - On [`RealDirectSubmission`]: verified roundtrip sets `ran_on_device_path=true`
///   and status `bo_roundtrip_ok` (silicon DMA path).
/// - On [`StubDirectSubmission`]: exercises control flow only; status
///   `bo_roundtrip_stub_ok` and **`ran_on_device_path=false`** even if bytes match.
/// - **Never** sets `ran_on_npu`.
///
/// Stage timing (exclusive intent):
/// - `host_prep_s` = CREATE_BO + host fill (write + SYNC TO_DEVICE inside write)
/// - `fill_s` = host fill only
/// - `device_s` = SYNC FROM_DEVICE only
/// - host mmap read after FROM is not double-SYNC'd (`read_bo_bytes_no_sync`)
/// - `total_s` = wall clock (not necessarily sum of stages)
///
/// Matrix fields `m`/`n`/`k` are **0**; size is in [`TimedOpResult::size_bytes`].
///
/// `size_bytes` of 0 uses [`DEFAULT_BO_ROUNDTRIP_SIZE`].
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
pub fn timed_bo_roundtrip_direct(
    submission: &dyn DirectSubmission,
    size_bytes: u64,
) -> TimedOpResult {
    use std::time::Instant;

    let size = if size_bytes == 0 {
        DEFAULT_BO_ROUNDTRIP_SIZE
    } else {
        size_bytes.max(16)
    };
    // Cap pathological sizes in unit tests / accidental huge requests.
    let size = size.min(16 * 1024 * 1024);
    let size_usize = usize::try_from(size).unwrap_or(usize::MAX);
    let real_ioctl = submission.is_real_ioctl_path();

    let t0 = Instant::now();
    let mut host_prep_s = 0.0_f64;

    let pattern: Vec<u8> = (0..size_usize)
        .map(|i| {
            let b = (i & 0xff) as u8;
            b.wrapping_mul(17).wrapping_add(0xA5)
        })
        .collect();

    let prep0 = Instant::now();
    let bo = submission.create_buffer(0, size, BufferType::Shmem);
    let create_elapsed = prep0.elapsed().as_secs_f64();
    host_prep_s += create_elapsed;

    let handle = match bo {
        Ok(h) => h,
        Err(e) => {
            let mut r = timed_op_base("bo_roundtrip", 0, 0, 0, "bytes", real_ioctl, Some(size));
            r.total_s = t0.elapsed().as_secs_f64();
            r.host_prep_s = host_prep_s;
            r.status = "create_bo_failed".to_string();
            r.detail = format!("CREATE_BO(Shmem, {size}) failed: {e}");
            return r;
        }
    };

    // Host fill only (write_bo_bytes includes SYNC TO_DEVICE on Real).
    let f0 = Instant::now();
    if let Err(e) = submission.write_bo_bytes(0, handle.0, 0, &pattern) {
        let _ = submission.destroy_buffer(0, handle.0);
        let fill_elapsed = f0.elapsed().as_secs_f64();
        host_prep_s += fill_elapsed;
        let mut r = timed_op_base("bo_roundtrip", 0, 0, 0, "bytes", real_ioctl, Some(size));
        r.total_s = t0.elapsed().as_secs_f64();
        r.host_prep_s = host_prep_s;
        r.fill_s = Some(fill_elapsed);
        r.status = "write_bo_failed".to_string();
        r.detail = format!("write_bo_bytes failed: {e}");
        return r;
    }
    let fill_elapsed = f0.elapsed().as_secs_f64();
    host_prep_s += fill_elapsed;

    // Device stage: SYNC FROM only (exclusive from fill).
    let d0 = Instant::now();
    if let Err(e) = submission.sync_buffer(0, handle, SYNC_BO_FROM_DEVICE, 0, size) {
        let _ = submission.destroy_buffer(0, handle.0);
        let device_elapsed = d0.elapsed().as_secs_f64();
        let mut r = timed_op_base("bo_roundtrip", 0, 0, 0, "bytes", real_ioctl, Some(size));
        r.total_s = t0.elapsed().as_secs_f64();
        r.host_prep_s = host_prep_s;
        r.fill_s = Some(fill_elapsed);
        r.device_s = Some(device_elapsed);
        r.status = "sync_from_device_failed".to_string();
        r.detail = format!("SYNC_BO FROM_DEVICE failed: {e}");
        return r;
    }
    let device_elapsed = d0.elapsed().as_secs_f64();

    // Host read without a second FROM_DEVICE sync.
    let read_back = match submission.read_bo_bytes_no_sync(0, handle.0, 0, size_usize) {
        Ok(b) => b,
        Err(e) => {
            let _ = submission.destroy_buffer(0, handle.0);
            let mut r = timed_op_base("bo_roundtrip", 0, 0, 0, "bytes", real_ioctl, Some(size));
            r.total_s = t0.elapsed().as_secs_f64();
            r.host_prep_s = host_prep_s;
            r.fill_s = Some(fill_elapsed);
            r.device_s = Some(device_elapsed);
            r.status = "read_bo_failed".to_string();
            r.detail = format!("read_bo_bytes_no_sync failed: {e}");
            return r;
        }
    };
    let _ = submission.destroy_buffer(0, handle.0);

    let mut r = timed_op_base("bo_roundtrip", 0, 0, 0, "bytes", real_ioctl, Some(size));
    r.total_s = t0.elapsed().as_secs_f64();
    r.host_prep_s = host_prep_s;
    r.fill_s = Some(fill_elapsed);
    r.device_s = Some(device_elapsed);
    r.ran_on_npu = false;

    if read_back == pattern {
        if real_ioctl {
            r.ran_on_device_path = true;
            r.status = "bo_roundtrip_ok".to_string();
            r.detail = format!(
                "CREATE_BO+fill+SYNC TO/FROM+verify OK ({size} bytes) via real DRM ioctls. \
                 ran_on_device_path=true; ran_on_npu=false (not GEMM)."
            );
        } else {
            // In-memory stub: never claim silicon DMA Pass.
            r.ran_on_device_path = false;
            r.status = "bo_roundtrip_stub_ok".to_string();
            r.detail = format!(
                "stub in-memory BO roundtrip OK ({size} bytes). \
                 ran_on_device_path=false (not silicon); ran_on_npu=false. \
                 Product Pass requires real_device_open + RealDirectSubmission."
            );
        }
    } else {
        r.ran_on_device_path = false;
        let mism = read_back
            .iter()
            .zip(pattern.iter())
            .filter(|(a, b)| a != b)
            .count();
        r.status = "bo_roundtrip_verify_failed".to_string();
        r.detail =
            format!("byte verify failed: {mism}/{size} mismatches (real_ioctl={real_ioctl})");
    }
    r
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
    ///
    /// **Must** hard-fail when [`exec_cmd_is_enabled`] is false (L1 double gate).
    fn submit_command(&self, fd: RawFd, cmd: &Command) -> Result<u64, DirectError>; // returns seq

    /// Wait for completion of a prior EXEC_CMD identified by `seq`.
    ///
    /// Direct equivalent of `DRM_IOCTL_AMDXDNA_WAIT_CMD` (`0xc010_6449`).
    ///
    /// **Not** gated by [`exec_cmd_is_enabled`]: wait without submit is useless
    /// but must not be blocked so completion/cleanup remains available after a
    /// lab single-shot. **Submit** stays gated.
    ///
    /// `timeout_ms == 0` is infinite (driver) and is **refused** unless
    /// `XDNA2_WAIT_ALLOW_INFINITE=1` ([`check_wait_timeout_or_infinite_opt_in`]).
    ///
    /// Default returns [`DirectError::Unsupported`]; Real/Stub override.
    fn wait_command(
        &self,
        fd: RawFd,
        hwctx: u32,
        seq: u64,
        timeout_ms: u32,
    ) -> Result<(), DirectError> {
        let _ = (fd, hwctx, seq, timeout_ms);
        Err(DirectError::Unsupported(
            "wait_command not implemented for this submission backend".to_string(),
        ))
    }

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

    /// Set power mode (`DRM_AMDXDNA_SET_POWER_MODE` via SET_STATE).
    ///
    /// Default: unsupported. Real path implements from uAPI; **operator-tested**
    /// only — unit tests cover packing/sizes, not live SET without hardware.
    fn set_power_mode(&self, mode: u8) -> Result<(), DirectError> {
        let _ = mode;
        Err(DirectError::Unsupported(
            "set_power_mode not implemented for this submission backend".to_string(),
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

    /// Write host bytes into a BO (mmap path on Real).
    ///
    /// **Required** before EXEC_CMD for Cmd BOs — see [`fill_cmd_bo_with_payload`].
    /// Default returns [`DirectError::Unsupported`].
    fn write_bo_bytes(
        &self,
        _fd: RawFd,
        _handle: u32,
        _offset: u64,
        _data: &[u8],
    ) -> Result<(), DirectError> {
        Err(DirectError::Unsupported(
            "write_bo_bytes not implemented for this submission backend".to_string(),
        ))
    }

    /// Read host-visible BO bytes (mmap path on Real after SYNC FROM_DEVICE).
    ///
    /// Used by [`timed_bo_roundtrip_direct`]. Default returns
    /// [`DirectError::Unsupported`].
    fn read_bo_bytes(
        &self,
        fd: RawFd,
        handle: u32,
        offset: u64,
        len: usize,
    ) -> Result<Vec<u8>, DirectError> {
        // Default: no-sync path if overridden; Real/Stub implement properly.
        self.read_bo_bytes_no_sync(fd, handle, offset, len)
    }

    /// Read BO bytes via mmap **without** SYNC FROM_DEVICE.
    ///
    /// Caller must already have coherency (e.g. after explicit FROM). Avoids
    /// double-SYNC when measuring exclusive `device_s` in roundtrip.
    fn read_bo_bytes_no_sync(
        &self,
        _fd: RawFd,
        _handle: u32,
        _offset: u64,
        _len: usize,
    ) -> Result<Vec<u8>, DirectError> {
        Err(DirectError::Unsupported(
            "read_bo_bytes_no_sync not implemented for this submission backend".to_string(),
        ))
    }

    /// True when this backend issues real DRM ioctls against an open device fd.
    ///
    /// [`StubDirectSubmission`] returns false. Product surfaces must not treat
    /// stub in-memory BO verify as `ran_on_device_path` silicon Pass.
    fn is_real_ioctl_path(&self) -> bool {
        false
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
///
/// Holds an in-memory BO store so unit tests can exercise
/// [`timed_bo_roundtrip_direct`] without `/dev/accel`.
#[derive(Debug)]
pub struct StubDirectSubmission {
    /// The raw fd we will eventually use for ioctls.
    /// Currently unused in the stub, but captured for API compatibility
    /// and to make the transition to real code trivial.
    _fd: RawFd,
    /// Handle → host bytes for write/read roundtrip unit tests.
    bo_bytes: std::sync::Mutex<std::collections::HashMap<u32, Vec<u8>>>,
    /// Monotonic handle allocator (starts at 42 for historical test fixtures).
    next_handle: std::sync::atomic::AtomicU32,
    /// GEM handles closed via [`DirectSubmission::destroy_buffer`] (test observability).
    destroyed_bos: std::sync::Mutex<Vec<u32>>,
}

impl Default for StubDirectSubmission {
    fn default() -> Self {
        Self::new(-1)
    }
}

impl StubDirectSubmission {
    /// Create a stub submission context for the given open fd.
    /// The fd is captured even though the stub does nothing with it yet.
    pub fn new(fd: RawFd) -> Self {
        Self {
            _fd: fd,
            bo_bytes: std::sync::Mutex::new(std::collections::HashMap::new()),
            next_handle: std::sync::atomic::AtomicU32::new(42),
            destroyed_bos: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Handles that were passed to [`DirectSubmission::destroy_buffer`] (unit tests).
    #[must_use]
    pub fn destroyed_bo_handles(&self) -> Vec<u32> {
        self.destroyed_bos
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
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
        // Allocate fake UMQ/log BOs so timed_matmul cleanup can be tested
        // (Strategy-B style non-zero handles; destroyed via destroy_hwctx_and_owned_bos).
        let umq = self
            .next_handle
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let log = self
            .next_handle
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if let Ok(mut map) = self.bo_bytes.lock() {
            map.insert(umq, vec![0u8; 4096]);
            map.insert(log, vec![0u8; 4096]);
        }
        Ok(HardwareContext {
            handle: 1,
            num_tiles,
            mem_size,
            umq_bo: umq,
            log_buf_bo: log,
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
        let _ = ty;
        let h = self
            .next_handle
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let cap = usize::try_from(size).unwrap_or(0).max(1);
        if let Ok(mut map) = self.bo_bytes.lock() {
            map.insert(h, vec![0u8; cap]);
        }
        Ok(BufferHandle(h))
    }

    fn destroy_buffer(&self, _fd: RawFd, handle: u32) -> Result<(), DirectError> {
        if let Ok(mut map) = self.bo_bytes.lock() {
            map.remove(&handle);
        }
        if let Ok(mut v) = self.destroyed_bos.lock() {
            v.push(handle);
        }
        Ok(())
    }

    fn get_bo_info(&self, _fd: RawFd, handle: u32) -> Result<amdxdna_drm_get_bo_info, DirectError> {
        // Fake non-zero xdna_addr so instruction-blob unit paths can proceed
        // without silicon (still never claims ran_on_npu).
        Ok(amdxdna_drm_get_bo_info {
            ext: 0,
            ext_flags: 0,
            handle,
            pad: 0,
            map_offset: 0x1000,
            vaddr: 0,
            xdna_addr: 0x1_0000_0000 | u64::from(handle),
        })
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

    fn wait_command(
        &self,
        _fd: RawFd,
        _hwctx: u32,
        _seq: u64,
        timeout_ms: u32,
    ) -> Result<(), DirectError> {
        // Wait is not EXEC_CMD-gated (see trait docs). Same infinite-timeout
        // lab policy as Real. Stub pretends success so unit tests can exercise
        // submit_and_wait gate logic without hardware.
        check_wait_timeout_or_infinite_opt_in(timeout_ms)?;
        Ok(())
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

    #[allow(clippy::significant_drop_tightening)]
    fn write_bo_bytes(
        &self,
        _fd: RawFd,
        handle: u32,
        offset: u64,
        data: &[u8],
    ) -> Result<(), DirectError> {
        if data.is_empty() {
            return Err(DirectError::InvalidArgument(
                "write_bo_bytes refuses empty data (empty Cmd BO is the wedge class)".to_string(),
            ));
        }
        let start = usize::try_from(offset).map_err(|_| {
            DirectError::InvalidArgument("stub write_bo_bytes offset too large".to_string())
        })?;
        let end = start.checked_add(data.len()).ok_or_else(|| {
            DirectError::InvalidArgument("stub write_bo_bytes range overflow".to_string())
        })?;
        let mut map = self
            .bo_bytes
            .lock()
            .map_err(|_| DirectError::Unsupported("stub bo_bytes mutex poisoned".to_string()))?;
        let buf = map.entry(handle).or_insert_with(|| vec![0u8; data.len()]);
        if end > buf.len() {
            buf.resize(end, 0);
        }
        buf[start..end].copy_from_slice(data);
        drop(map);
        Ok(())
    }

    #[allow(clippy::significant_drop_tightening)]
    fn read_bo_bytes_no_sync(
        &self,
        _fd: RawFd,
        handle: u32,
        offset: u64,
        len: usize,
    ) -> Result<Vec<u8>, DirectError> {
        if len == 0 {
            return Err(DirectError::InvalidArgument(
                "read_bo_bytes_no_sync refuses len=0".to_string(),
            ));
        }
        let start = usize::try_from(offset).map_err(|_| {
            DirectError::InvalidArgument("stub read_bo_bytes offset too large".to_string())
        })?;
        let end = start.checked_add(len).ok_or_else(|| {
            DirectError::InvalidArgument("stub read_bo_bytes range overflow".to_string())
        })?;
        let map = self
            .bo_bytes
            .lock()
            .map_err(|_| DirectError::Unsupported("stub bo_bytes mutex poisoned".to_string()))?;
        let buf = map.get(&handle).ok_or_else(|| {
            DirectError::InvalidArgument(format!("stub read_bo_bytes: unknown handle {handle}"))
        })?;
        if end > buf.len() {
            return Err(DirectError::InvalidArgument(format!(
                "stub read_bo_bytes out of range end={end} len={}",
                buf.len()
            )));
        }
        let out = buf[start..end].to_vec();
        drop(map);
        Ok(out)
    }

    fn is_real_ioctl_path(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_submission_returns_clear_error_on_submit() {
        let stub = StubDirectSubmission::new(-1);
        let cmd = Command {
            ty: CommandType::SubmitExecBuf,
            hwctx_handle: 1,
            payload: vec![],
            cmd_bo_handle: None,
            arg_bo_handles: Vec::new(),
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
        let stub = StubDirectSubmission::new(-1);
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
        let stub = StubDirectSubmission::new(-1);
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
    fn set_power_mode_struct_sizes_and_packing() {
        // Header: set_power_mode is 8 bytes; set_state is 16 (same as get_info).
        assert_eq!(core::mem::size_of::<amdxdna_drm_set_power_mode>(), 8);
        assert_eq!(core::mem::size_of::<amdxdna_drm_set_state>(), 16);
        assert_eq!(
            core::mem::size_of::<amdxdna_drm_get_power_mode>(),
            core::mem::size_of::<amdxdna_drm_set_power_mode>()
        );
        assert_eq!(AMDXDNA_SET_PARAM_POWER_MODE, 0);
        assert_eq!(power_mode::TURBO, 4);
        // SET_STATE ioctl: nr=8, size=16, IOWR → 0xc010_6448
        #[cfg(feature = "amd-xdna2")]
        {
            assert_eq!(ioctl::DRM_IOCTL_AMDXDNA_SET_STATE, 0xc010_6448);
        }
    }

    #[test]
    fn set_power_mode_rejects_out_of_range_without_ioctl() {
        let real = RealDirectSubmission::new(-1);
        let err = real.set_power_mode(99).expect_err("range");
        assert!(
            matches!(err, DirectError::InvalidArgument(_)),
            "got {err:?}"
        );
    }

    #[test]
    fn suggest_hwctx_tiles_multiples_of_four() {
        assert_eq!(suggest_hwctx_tiles_from_tops(0), 4);
        assert_eq!(suggest_hwctx_tiles_from_tops(10), 4);
        assert_eq!(suggest_hwctx_tiles_from_tops(39), 4);
        assert_eq!(suggest_hwctx_tiles_from_tops(40), 8); // Strix-class
        assert_eq!(suggest_hwctx_tiles_from_tops(58), 8);
        let res = amdxdna_drm_get_resource_info {
            npu_clk_max: 1800,
            npu_tops_max: 58,
            npu_task_max: 16,
            npu_tops_curr: 0,
            npu_task_curr: 0,
        };
        let t = suggest_hwctx_tiles(&res);
        assert_eq!(t, 8);
        assert_eq!(t % 4, 0);
        assert!((DEFAULT_HWCTX_TILES..=MAX_SUGGESTED_HWCTX_TILES).contains(&t));
    }

    #[test]
    fn efficiency_notes_bo_roundtrip_dma_only() {
        let r = timed_bo_roundtrip_direct(&StubDirectSubmission::new(-1), 64);
        let notes = r.efficiency_notes();
        assert!(notes.contains("bo_roundtrip"), "{notes}");
        assert!(notes.contains("ran_on_npu=false"), "{notes}");
        // Issue 5: stub must not claim driver DMA.
        assert!(
            notes.contains("stub") && notes.contains("not driver DMA"),
            "stub notes must not claim DMA-only: {notes}"
        );
        assert!(
            !notes.contains("DMA-only"),
            "stub must not use DMA-only label: {notes}"
        );
        assert!(notes.contains("honesty"), "{notes}");
        assert!(!r.ran_on_npu);
        assert!(!r.ran_on_device_path); // stub
    }

    #[test]
    fn stub_set_power_mode_unsupported() {
        let stub = StubDirectSubmission::new(-1);
        assert!(matches!(
            stub.set_power_mode(power_mode::DEFAULT),
            Err(DirectError::Unsupported(_))
        ));
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
        let stub = StubDirectSubmission::new(-1);
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
            arg_bo_handles: Vec::new(),
        };

        assert_eq!(cmd.cmd_bo_handle, Some(123));

        let stub = StubDirectSubmission::new(-1);
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
        // packed header (4) + 4 zero data dwords (16)
        assert_eq!(p.len(), 4 + 16);
        let hdr = u32::from_le_bytes([p[0], p[1], p[2], p[3]]);
        let fields = unpack_amdxdna_cmd_header(hdr);
        assert_eq!(fields.state, ErtCmdState::New as u32);
        assert_eq!(fields.count, 4);
        assert_eq!(fields.opcode, 0);
    }

    #[test]
    fn pack_unpack_amdxdna_cmd_header_roundtrip() {
        let h = pack_amdxdna_cmd_header(1, 2, 0x123, ErtCmdOpcode::StartNpu as u32);
        let f = unpack_amdxdna_cmd_header(h);
        assert_eq!(f.state, 1);
        assert_eq!(f.extra_cu_mask, 2);
        assert_eq!(f.count, 0x123);
        assert_eq!(f.opcode, ErtCmdOpcode::StartNpu as u32);
    }

    #[test]
    fn pack_unpack_amdxdna_cmd_header_max_masks() {
        // Max representable field values (masks truncate wider inputs).
        let h = pack_amdxdna_cmd_header(0xf, 0x3, 0x7ff, 0x1f);
        let f = unpack_amdxdna_cmd_header(h);
        assert_eq!(f.state, 0xf);
        assert_eq!(f.extra_cu_mask, 0x3);
        assert_eq!(f.count, 0x7ff);
        assert_eq!(f.opcode, 0x1f);
        // Overflow is masked (not panicking).
        let h2 = pack_amdxdna_cmd_header(0x10, 0x4, 0x800, 0x20);
        let f2 = unpack_amdxdna_cmd_header(h2);
        assert_eq!(f2.state, 0);
        assert_eq!(f2.extra_cu_mask, 0);
        assert_eq!(f2.count, 0);
        assert_eq!(f2.opcode, 0);
        // ERT_INVALID_CMD (!0) packs to 5-bit max, not all-ones.
        let inv = pack_amdxdna_cmd_header(1, 0, 1, ErtCmdOpcode::Invalid as u32);
        assert_eq!(unpack_amdxdna_cmd_header(inv).opcode, 0x1f);
    }

    #[test]
    fn parse_amdxdna_cmd_header_from_bytes_little_endian() {
        let h = pack_amdxdna_cmd_header(
            ErtCmdState::Completed as u32,
            1,
            42,
            ErtCmdOpcode::StartNpu as u32,
        );
        let mut bytes = h.to_le_bytes().to_vec();
        bytes.extend_from_slice(&[0u8; 8]);
        let f = parse_amdxdna_cmd_header_from_bytes(&bytes).expect("header parse");
        assert_eq!(f.state, ErtCmdState::Completed as u32);
        assert_eq!(f.extra_cu_mask, 1);
        assert_eq!(f.count, 42);
        assert_eq!(f.opcode, ErtCmdOpcode::StartNpu as u32);
        // Short buffer fails closed.
        assert!(parse_amdxdna_cmd_header_from_bytes(&bytes[..3]).is_err());
        assert!(parse_amdxdna_cmd_header_from_bytes(&[]).is_err());
        // Bare 4-byte LE header also works.
        let f2 = parse_amdxdna_cmd_header_from_bytes(&h.to_le_bytes()).expect("4-byte header");
        assert_eq!(f2, f);
    }

    #[test]
    fn amdxdna_cmd_num_cu_masks_chain_vs_start() {
        assert_eq!(
            amdxdna_cmd_num_cu_masks(ErtCmdOpcode::CmdChain as u32, 3),
            0
        );
        assert_eq!(
            amdxdna_cmd_num_cu_masks(ErtCmdOpcode::StartNpu as u32, 0),
            1
        );
        assert_eq!(
            amdxdna_cmd_num_cu_masks(ErtCmdOpcode::StartNpu as u32, 2),
            3
        );
    }

    #[test]
    fn build_ert_start_npu_cmd_bo_layout() {
        let bo = build_ert_start_npu_cmd_bo(0x1111_2222_3333_4444, 0x1000, &[7, 8], 0x1)
            .expect("START_NPU build");
        assert!(bo.len() >= 4 + 4 * 6);
        let f = parse_amdxdna_cmd_header_from_bytes(&bo).expect("header");
        assert_eq!(f.opcode, ErtCmdOpcode::StartNpu as u32);
        assert_eq!(f.state, ErtCmdState::New as u32);
        assert_eq!(f.extra_cu_mask, 0);
        // count = 1 cu + 2 buffer + size + prop_count + 2 props = 7
        assert_eq!(f.count, 7);
        assert_eq!(amdxdna_cmd_num_cu_masks(f.opcode, f.extra_cu_mask), 1);
        // COUNT matches body dword count
        assert_eq!(bo.len(), 4 + (f.count as usize) * 4);
        // cu_mask0
        assert_eq!(u32::from_le_bytes([bo[4], bo[5], bo[6], bo[7]]), 0x1);
        // buffer low/high
        assert_eq!(
            u32::from_le_bytes([bo[8], bo[9], bo[10], bo[11]]),
            0x3333_4444
        );
        assert_eq!(
            u32::from_le_bytes([bo[12], bo[13], bo[14], bo[15]]),
            0x1111_2222
        );
        assert_eq!(u32::from_le_bytes([bo[16], bo[17], bo[18], bo[19]]), 0x1000);
        assert_eq!(u32::from_le_bytes([bo[20], bo[21], bo[22], bo[23]]), 2);
        assert_eq!(u32::from_le_bytes([bo[24], bo[25], bo[26], bo[27]]), 7);
        assert_eq!(u32::from_le_bytes([bo[28], bo[29], bo[30], bo[31]]), 8);
    }

    #[test]
    fn build_ert_start_npu_cmd_bo_empty_prop_args() {
        let bo = build_ert_start_npu_cmd_bo(0xaaaa_bbbb_cccc_dddd, 64, &[], 0x1)
            .expect("START_NPU empty props");
        let f = parse_amdxdna_cmd_header_from_bytes(&bo).expect("header");
        assert_eq!(f.opcode, ErtCmdOpcode::StartNpu as u32);
        // 1 cu + 2 buffer + size + prop_count = 5
        assert_eq!(f.count, 5);
        assert_eq!(bo.len(), 4 + 5 * 4);
        assert_eq!(u32::from_le_bytes([bo[20], bo[21], bo[22], bo[23]]), 0); // prop_count
    }

    #[test]
    fn build_ert_start_npu_cmd_bo_with_kernel_args_prop_count_zero() {
        let kargs = super::super::lab_io::pack_start_npu_kernel_args(0x10, 0x20, 0x30);
        assert_eq!(kargs.len(), 15);
        let bo = build_ert_start_npu_cmd_bo_with_kernel_args(0x1000, 256, &kargs, 0x1)
            .expect("kernel args START_NPU");
        let f = parse_amdxdna_cmd_header_from_bytes(&bo).expect("header");
        assert_eq!(f.opcode, ErtCmdOpcode::StartNpu as u32);
        // 1 cu + 2 buffer + size + prop_count + 15 kernel = 20
        assert_eq!(f.count, 20);
        assert_eq!(bo.len(), 4 + 20 * 4);
        // prop_count field at dword after size must be 0 (kernel args, not props)
        assert_eq!(u32::from_le_bytes([bo[20], bo[21], bo[22], bo[23]]), 0);
        // first kernel dword = 3 (marker low)
        assert_eq!(u32::from_le_bytes([bo[24], bo[25], bo[26], bo[27]]), 3);
        // IFM lo at expected slot (after 3,0 u64 + 0 u32 = 5 dwords into args)
        // args start at byte 24; IFM lo is args[5] → byte 24+20=44
        assert_eq!(u32::from_le_bytes([bo[44], bo[45], bo[46], bo[47]]), 0x10);
    }

    #[test]
    fn legacy_builder_vs_kernel_args_prop_count_paired() {
        // Same payload dwords: legacy treats ALL as properties (prop_count=len);
        // with_kernel_args keeps prop_count=0 (shim-compatible I/O bind).
        let dwords = [7u32, 8, 9];
        let legacy = build_ert_start_npu_cmd_bo(0x100, 64, &dwords, 0x1).expect("legacy");
        let kargs =
            build_ert_start_npu_cmd_bo_with_kernel_args(0x100, 64, &dwords, 0x1).expect("kernel");
        // Same total size / COUNT (1+2+1+1+3 = 8)
        assert_eq!(legacy.len(), kargs.len());
        let fl = parse_amdxdna_cmd_header_from_bytes(&legacy).unwrap();
        let fk = parse_amdxdna_cmd_header_from_bytes(&kargs).unwrap();
        assert_eq!(fl.count, fk.count);
        assert_eq!(fl.count, 8);
        // prop_count field differs
        assert_eq!(
            u32::from_le_bytes([legacy[20], legacy[21], legacy[22], legacy[23]]),
            3
        );
        assert_eq!(
            u32::from_le_bytes([kargs[20], kargs[21], kargs[22], kargs[23]]),
            0
        );
        // Flexible-array payload dwords still match after prop_count field
        assert_eq!(&legacy[24..], &kargs[24..]);
    }

    #[test]
    fn build_ert_start_npu_cmd_bo_ex_prop_count_vs_args_len() {
        // 2 props + 2 kernel-style trailing dwords; prop_count=2
        let args = [1u32, 2, 3, 4];
        let bo = build_ert_start_npu_cmd_bo_ex(0x55, 8, 2, &args, 0x1).expect("ex");
        assert_eq!(u32::from_le_bytes([bo[20], bo[21], bo[22], bo[23]]), 2);
        let f = parse_amdxdna_cmd_header_from_bytes(&bo).expect("header");
        // 1+2+1+1+4 = 9
        assert_eq!(f.count, 9);
        let err = build_ert_start_npu_cmd_bo_ex(0, 0, 5, &[1, 2], 0x1).expect_err("prop_count");
        assert!(err.to_string().contains("prop_count"), "err={err}");
    }

    #[test]
    fn build_ert_start_npu_cmd_bo_count_overflow_fails() {
        // Fixed overhead = 5 dwords; max props = 0x7ff - 5. One more → fail closed.
        let too_many = vec![0u32; (AMDXDNA_CMD_COUNT_MAX as usize) - 5 + 1];
        let err = build_ert_start_npu_cmd_bo(0, 0, &too_many, 0x1).expect_err("COUNT overflow");
        assert!(
            matches!(err, DirectError::InvalidArgument(_)),
            "err={err:?}"
        );
        let msg = err.to_string();
        assert!(
            msg.contains("COUNT") || msg.contains("overflow"),
            "msg={msg}"
        );
    }

    #[test]
    fn build_ert_cmd_chain_bo_handles() {
        let bo = build_ert_cmd_chain_bo(&[0xa, 0xb]).expect("CHAIN build");
        let f = parse_amdxdna_cmd_header_from_bytes(&bo).expect("header");
        assert_eq!(f.opcode, ErtCmdOpcode::CmdChain as u32);
        assert_eq!(amdxdna_cmd_num_cu_masks(f.opcode, f.extra_cu_mask), 0);
        // 6 fixed + 2*2 handle dwords = 10
        assert_eq!(f.count, 10);
        assert_eq!(bo.len(), 4 + (f.count as usize) * 4);
        assert_eq!(u32::from_le_bytes([bo[4], bo[5], bo[6], bo[7]]), 2); // command_count
        // handle 0 = 0xa
        assert_eq!(u32::from_le_bytes([bo[28], bo[29], bo[30], bo[31]]), 0xa);
        assert_eq!(u32::from_le_bytes([bo[32], bo[33], bo[34], bo[35]]), 0);
        // handle 1 = 0xb
        assert_eq!(u32::from_le_bytes([bo[36], bo[37], bo[38], bo[39]]), 0xb);
    }

    #[test]
    fn build_ert_cmd_chain_bo_empty_list() {
        let bo = build_ert_cmd_chain_bo(&[]).expect("empty CHAIN");
        let f = parse_amdxdna_cmd_header_from_bytes(&bo).expect("header");
        assert_eq!(f.opcode, ErtCmdOpcode::CmdChain as u32);
        assert_eq!(f.count, 6); // fixed chain header only
        assert_eq!(bo.len(), 4 + 6 * 4);
        assert_eq!(u32::from_le_bytes([bo[4], bo[5], bo[6], bo[7]]), 0); // command_count
    }

    #[test]
    fn build_ert_start_npu_preempt_cmd_bo_layout() {
        let bo = build_ert_start_npu_preempt_cmd_bo(
            0x1000,
            0x2000,
            0x3000,
            0x10,
            0x20,
            0x30,
            &[9],
            0x1,
            ErtCmdOpcode::StartNpuPreempt,
        )
        .expect("PREEMPT build");
        let f = parse_amdxdna_cmd_header_from_bytes(&bo).expect("header");
        assert_eq!(f.opcode, ErtCmdOpcode::StartNpuPreempt as u32);
        assert_eq!(f.state, ErtCmdState::New as u32);
        // 1 cu + 6 u64 halves + 4 fixed + 1 prop = 12
        assert_eq!(f.count, 12);
        assert_eq!(bo.len(), 4 + 12 * 4);
        assert_eq!(u32::from_le_bytes([bo[4], bo[5], bo[6], bo[7]]), 0x1);
        assert_eq!(u32::from_le_bytes([bo[8], bo[9], bo[10], bo[11]]), 0x1000); // inst low
        assert_eq!(u32::from_le_bytes([bo[16], bo[17], bo[18], bo[19]]), 0x2000); // save low
        assert_eq!(u32::from_le_bytes([bo[24], bo[25], bo[26], bo[27]]), 0x3000); // restore low
        // sizes start after 1+6 dwords = offset 4+7*4 = 32
        assert_eq!(u32::from_le_bytes([bo[32], bo[33], bo[34], bo[35]]), 0x10);
        assert_eq!(u32::from_le_bytes([bo[36], bo[37], bo[38], bo[39]]), 0x20);
        assert_eq!(u32::from_le_bytes([bo[40], bo[41], bo[42], bo[43]]), 0x30);
        assert_eq!(u32::from_le_bytes([bo[44], bo[45], bo[46], bo[47]]), 1); // inst_prop_cnt
        assert_eq!(u32::from_le_bytes([bo[48], bo[49], bo[50], bo[51]]), 9);

        let elf = build_ert_start_npu_preempt_elf_cmd_bo(0, 0, 0, 0, 0, 0, &[], 0x1)
            .expect("PREEMPT_ELF build");
        let fe = parse_amdxdna_cmd_header_from_bytes(&elf).expect("elf header");
        assert_eq!(fe.opcode, ErtCmdOpcode::StartNpuPreemptElf as u32);
    }

    #[test]
    fn build_ert_start_npu_preempt_rejects_wrong_opcode() {
        let err =
            build_ert_start_npu_preempt_cmd_bo(0, 0, 0, 0, 0, 0, &[], 0x1, ErtCmdOpcode::StartNpu)
                .expect_err("wrong opcode");
        assert!(matches!(err, DirectError::InvalidArgument(_)));
        assert!(err.to_string().contains("StartNpuPreempt"));
    }

    #[test]
    fn build_ert_start_dpu_cmd_bo_layout() {
        let bo = build_ert_start_dpu_cmd_bo(0x111, 0x2222_3333_4444_5555, 0x100, 3, 0, 0x1)
            .expect("START_DPU build");
        let f = parse_amdxdna_cmd_header_from_bytes(&bo).expect("header");
        assert_eq!(f.opcode, ErtCmdOpcode::StartDpu as u32);
        assert_eq!(f.count, 7);
        assert_eq!(bo.len(), 4 + 7 * 4);
        assert_eq!(u32::from_le_bytes([bo[8], bo[9], bo[10], bo[11]]), 0x111); // dtrace low
        assert_eq!(
            u32::from_le_bytes([bo[16], bo[17], bo[18], bo[19]]),
            0x4444_5555
        ); // inst low
        assert_eq!(
            u32::from_le_bytes([bo[20], bo[21], bo[22], bo[23]]),
            0x2222_3333
        ); // inst high
        assert_eq!(u32::from_le_bytes([bo[24], bo[25], bo[26], bo[27]]), 0x100); // size
        // uc_index=3, chained=0 packed
        assert_eq!(u32::from_le_bytes([bo[28], bo[29], bo[30], bo[31]]), 3);
    }

    #[test]
    fn build_ert_start_dpu_cmd_bo_rejects_nonzero_chained() {
        let err = build_ert_start_dpu_cmd_bo(0, 0, 0, 0, 1, 0x1).expect_err("chained!=0");
        assert!(matches!(err, DirectError::InvalidArgument(_)));
        assert!(err.to_string().contains("chained"));
    }

    #[test]
    fn amdxdna_drm_wait_cmd_struct_size_is_16_bytes() {
        assert_eq!(core::mem::size_of::<amdxdna_drm_wait_cmd>(), 16);
        assert_eq!(core::mem::align_of::<amdxdna_drm_wait_cmd>(), 8);
    }

    #[test]
    fn drm_ioctl_amdxdna_wait_cmd_is_iowr_size_16() {
        // Regression: must be DRM_IOWR (dir=3), not DRM_IOW (dir=1).
        // Matches drm_local/amdxdna_accel.h used by the amdxdna driver.
        assert_eq!(
            ioctl::DRM_IOCTL_AMDXDNA_WAIT_CMD,
            ioctl::iowr::<amdxdna_drm_wait_cmd>(9)
        );
        assert_eq!(ioctl::DRM_IOCTL_AMDXDNA_WAIT_CMD, 0xc010_6449);
        // Explicitly not the historical wrong IOW encoding.
        assert_ne!(ioctl::DRM_IOCTL_AMDXDNA_WAIT_CMD, 0x4010_6449);
    }

    #[test]
    fn build_exec_cmd_payload_scaffold_clamps_body_to_count() {
        // Body request larger than 11-bit COUNT → clamp both COUNT and data[] together.
        let p = build_exec_cmd_payload_scaffold(AMDXDNA_CMD_COUNT_MAX as usize + 100);
        let f = parse_amdxdna_cmd_header_from_bytes(&p).expect("header");
        assert_eq!(f.count, AMDXDNA_CMD_COUNT_MAX);
        assert_eq!(p.len(), 4 + (AMDXDNA_CMD_COUNT_MAX as usize) * 4);
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
            arg_bo_handles: Vec::new(),
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
            arg_bo_handles: Vec::new(),
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
        assert!(!r.ran_on_device_path);
        assert!(r.hwctx_s.is_some(), "hwctx stage should be timed");
        // Branch on gate so a deliberate dangerous+ALLOW process still has honest asserts
        // (lib tests normally run with gate closed → hwctx_ok).
        if exec_cmd_is_enabled() {
            // Stub submit is attempted after host fill; never claim NPU Pass.
            // Stub submit_command returns Unsupported even when gates open →
            // payload_unsupported (or cmd_bo / fill / build / inst failures).
            assert!(
                r.status == "payload_unsupported"
                    || r.status == "submit_wait_failed"
                    || r.status == "wait_timeout"
                    || r.status == "exec_cmd_perm"
                    || r.status == "ok_submit_wait_envelope"
                    || r.status == "ok_submit_wait_with_inst_blob"
                    || r.status == "cmd_bo_failed"
                    || r.status == "cmd_bo_fill_failed"
                    || r.status == "payload_build_failed"
                    || r.status.starts_with("inst_blob_"),
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
        assert_eq!(v["schema_version"], crate::TIMED_OP_JSON_SCHEMA_VERSION);
        assert_eq!(v["op"], "matmul");
        assert_eq!(v["m"], 1);
        assert_eq!(v["ran_on_npu"], false);
        assert_eq!(v["ran_on_device_path"], false);
        assert!(v.get("hwctx_s").is_some());
        assert!(v.get("host_ref_ok").is_some());
        assert!(v.get("efficiency_notes").is_some());
    }

    #[test]
    fn timed_matmul_destroys_umq_and_log_bos_on_stub() {
        // Issue 2: after CREATE_HWCTX, UMQ/log must be GEM_CLOSEd via destroy_hwctx_and_owned_bos.
        let stub = StubDirectSubmission::new(-1);
        // Peek the handles stub will allocate: next_handle starts at 42 → umq=42, log=43.
        let r = timed_matmul_direct(&stub, 1, 1, 1, "f32");
        assert!(!r.ran_on_npu);
        let closed = stub.destroyed_bo_handles();
        assert!(
            closed.contains(&42) && closed.contains(&43),
            "expected UMQ+log BO close (42,43), got destroyed={closed:?}"
        );
    }

    #[test]
    fn destroy_hwctx_and_owned_bos_closes_nonzero_handles() {
        let stub = StubDirectSubmission::new(-1);
        let ctx = stub.create_hwctx(0, 4, 1024, None).expect("stub create");
        assert_ne!(ctx.umq_bo, 0);
        assert_ne!(ctx.log_buf_bo, 0);
        let umq = ctx.umq_bo;
        let log = ctx.log_buf_bo;
        destroy_hwctx_and_owned_bos(&stub, &ctx);
        let closed = stub.destroyed_bo_handles();
        assert!(
            closed.contains(&umq) && closed.contains(&log),
            "got {closed:?}"
        );
    }

    #[test]
    fn host_matmul_f32_ones_checksum_stable() {
        let r = host_matmul_f32_ones(2, 3, 4).expect("host matmul");
        assert!((r.c00 - 4.0).abs() < 1e-5);
        assert_eq!(r.out_elems, 6);
        assert!(r.elapsed_s >= 0.0);
        // Same inputs → same checksum.
        let r2 = host_matmul_f32_ones(2, 3, 4).expect("host matmul 2");
        assert_eq!(r.checksum, r2.checksum);
    }

    #[test]
    fn host_matmul_refuses_zero_dims() {
        assert!(host_matmul_f32_ones(0, 1, 1).is_err());
        assert!(host_matmul_f32_ones(1, 0, 1).is_err());
        assert!(host_matmul_f32_ones(1, 1, 0).is_err());
    }

    #[test]
    fn decode_instruction_hex_roundtrip() {
        let b = decode_instruction_hex("0xDEAD beef").expect("hex");
        assert_eq!(b, vec![0xDE, 0xAD, 0xBE, 0xEF]);
        assert!(decode_instruction_hex("xyz").is_err());
        assert!(decode_instruction_hex("abc").is_err()); // odd length
        assert!(decode_instruction_hex("").is_err());
    }

    #[test]
    fn load_instruction_blob_none_when_unset() {
        // Pure: only assert when both env vars are unset (no process mutation).
        let blob_set = std::env::var("XDNA2_INSTRUCTION_BLOB").is_ok();
        let hex_set = std::env::var("XDNA2_INSTRUCTION_HEX").is_ok();
        if !blob_set && !hex_set {
            let v = load_instruction_blob_from_env().expect("unset ok");
            assert!(v.is_none());
        }
    }

    #[test]
    fn load_instruction_blob_from_path_missing_fails() {
        let err = load_instruction_blob_from_path("/nonexistent/dragon_npu_inst_blob_xyz.bin")
            .expect_err("missing file");
        let s = err.to_string();
        assert!(
            s.contains("metadata failed")
                || s.contains("read failed")
                || s.contains("No such")
                || s.contains("not found"),
            "err={err}"
        );
    }

    #[test]
    fn load_instruction_blob_from_path_refuses_oversize() {
        assert_eq!(MAX_INSTRUCTION_BLOB_BYTES, 64 * 1024 * 1024);
        // Sparse file: metadata.len() > cap without writing 64MiB of data.
        let dir = std::env::temp_dir();
        let path = dir.join("dragon_npu_oversize_inst_blob_test.bin");
        let f = std::fs::File::create(&path).expect("create sparse");
        f.set_len(MAX_INSTRUCTION_BLOB_BYTES + 1)
            .expect("set_len oversize");
        drop(f);
        let err = load_instruction_blob_from_path(path.to_str().unwrap_or(""))
            .expect_err("oversize must refuse before full read");
        let _ = std::fs::remove_file(&path);
        let msg = err.to_string();
        assert!(
            msg.contains("max") || msg.contains('>') || msg.contains("bytes"),
            "err={msg}"
        );
    }

    #[test]
    fn load_instruction_blob_from_path_empty_fails() {
        let dir = std::env::temp_dir();
        let path = dir.join("dragon_npu_empty_inst_blob_test.bin");
        std::fs::write(&path, b"").expect("write empty");
        let err = load_instruction_blob_from_path(path.to_str().unwrap_or("")).expect_err("empty");
        let _ = std::fs::remove_file(&path);
        assert!(err.to_string().contains("empty"), "err={err}");
    }

    #[test]
    fn load_instruction_blob_from_path_ok() {
        let dir = std::env::temp_dir();
        let path = dir.join("dragon_npu_inst_blob_test.bin");
        std::fs::write(&path, [0xde, 0xad, 0xbe, 0xef]).expect("write");
        let bytes =
            load_instruction_blob_from_path(path.to_str().unwrap_or("")).expect("load path");
        let _ = std::fs::remove_file(&path);
        assert_eq!(bytes, vec![0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn timed_bo_roundtrip_on_stub_verifies_pattern_without_device_pass() {
        let stub = StubDirectSubmission::new(-1);
        assert!(!stub.is_real_ioctl_path());
        let r = timed_bo_roundtrip_direct(&stub, 256);
        assert_eq!(r.op, "bo_roundtrip");
        assert!(!r.ran_on_npu);
        // Stub in-memory OK must never be product silicon DMA Pass.
        assert_eq!(r.status, "bo_roundtrip_stub_ok");
        assert!(!r.ran_on_device_path);
        assert!(!r.real_ioctl);
        assert_eq!(r.m, 0);
        assert_eq!(r.n, 0);
        assert_eq!(r.k, 0);
        assert_eq!(r.size_bytes, Some(256));
        assert!(r.total_s >= 0.0);
        assert!(r.fill_s.is_some());
        assert!(r.device_s.is_some());
        // Exclusive intent: host_prep should be create+fill, not include claiming device Pass.
        let v = r.to_json_value();
        assert_eq!(v["ran_on_npu"], false);
        assert_eq!(v["ran_on_device_path"], false);
        assert_eq!(v["real_ioctl"], false);
        assert_eq!(v["status"], "bo_roundtrip_stub_ok");
        assert_eq!(v["size_bytes"], 256);
    }

    #[test]
    fn product_cli_must_not_treat_stub_as_silicon_device_path() {
        // Documents Issue 1: exit 0 / pass_device_path requires real_ioctl + ran_on_device_path.
        let stub = StubDirectSubmission::new(-1);
        let r = timed_bo_roundtrip_direct(&stub, 64);
        let product_device_pass = r.real_ioctl && r.ran_on_device_path;
        assert!(
            !product_device_pass,
            "stub verify must not count as silicon DMA Pass"
        );
    }

    #[test]
    fn timed_matmul_statuses_never_claim_ran_on_npu_on_stub() {
        let stub = StubDirectSubmission::new(-1);
        for (m, n, k) in [(1u32, 1, 1), (1, 64, 64), (2, 2, 2)] {
            let r = timed_matmul_direct(&stub, m, n, k, "f32");
            assert!(
                !r.ran_on_npu,
                "ran_on_npu must stay false: status={}",
                r.status
            );
            // host_ref auto for small f32 shapes
            if m * n * k <= 256 * 1024 {
                assert!(
                    r.host_ref_ok || r.host_ref_detail.contains("host_ref"),
                    "expected host_ref attempt: {}",
                    r.host_ref_detail
                );
            }
        }
    }

    #[test]
    fn submit_and_wait_refuses_when_exec_cmd_gated() {
        assert!(
            !exec_cmd_is_enabled(),
            "submit_and_wait gate test requires EXEC_CMD gated closed \
             (unset XDNA2_ALLOW_EXEC_CMD; do not pair dangerous feature+ALLOW=1)"
        );
        let stub = StubDirectSubmission::new(-1);
        let cmd = Command {
            ty: CommandType::SubmitExecBuf,
            hwctx_handle: 1,
            payload: vec![0u8; 16],
            cmd_bo_handle: Some(99),
            arg_bo_handles: Vec::new(),
        };
        let err = submit_and_wait_command(&stub, 0, &cmd, 1000).unwrap_err();
        match err {
            DirectError::Unsupported(msg) => assert_eq!(msg, exec_cmd_block_reason()),
            other => panic!("expected Unsupported gate error, got {other:?}"),
        }
    }

    #[test]
    fn real_submit_and_wait_refuses_when_exec_cmd_gated() {
        assert!(
            !exec_cmd_is_enabled(),
            "real submit_and_wait gate test requires gated process (fake fd; no ioctl)"
        );
        let real = RealDirectSubmission::new(42);
        let cmd = Command {
            ty: CommandType::SubmitExecBuf,
            hwctx_handle: 1,
            payload: vec![0u8; 16],
            cmd_bo_handle: Some(99),
            arg_bo_handles: Vec::new(),
        };
        let err = submit_and_wait_command(&real, 0, &cmd, 500).unwrap_err();
        match err {
            DirectError::Unsupported(msg) => assert_eq!(msg, exec_cmd_block_reason()),
            other => panic!("expected Unsupported gate error, got {other:?}"),
        }
    }

    #[test]
    fn submit_and_wait_refuses_zero_timeout() {
        // Even if EXEC_CMD were enabled, infinite wait is refused by the helper.
        // When gated, gate error wins first — so only assert InvalidArgument when
        // gates are open; when closed, assert Unsupported (gate takes precedence).
        // Pure policy coverage: `check_finite_wait_timeout_refuses_zero` (no ALLOW).
        let stub = StubDirectSubmission::new(-1);
        let cmd = Command {
            ty: CommandType::SubmitExecBuf,
            hwctx_handle: 1,
            payload: vec![],
            cmd_bo_handle: None,
            arg_bo_handles: Vec::new(),
        };
        let err = submit_and_wait_command(&stub, 0, &cmd, 0).unwrap_err();
        if exec_cmd_is_enabled() {
            assert!(
                matches!(err, DirectError::InvalidArgument(_)),
                "expected InvalidArgument for timeout_ms=0, got {err:?}"
            );
            assert!(
                err.to_string().contains("timeout_ms=0") || err.to_string().contains("infinite"),
                "err={err}"
            );
        } else {
            match err {
                DirectError::Unsupported(msg) => assert_eq!(msg, exec_cmd_block_reason()),
                other => panic!("gated path should refuse before timeout check: {other:?}"),
            }
        }
    }

    #[test]
    fn check_finite_wait_timeout_refuses_zero() {
        // Pure unit coverage of lab timeout policy without L1 ALLOW or /dev/accel.
        let err = check_finite_wait_timeout(0).expect_err("0 must fail");
        assert!(matches!(err, DirectError::InvalidArgument(_)));
        assert!(
            err.to_string().contains("timeout_ms=0") || err.to_string().contains("infinite"),
            "err={err}"
        );
        check_finite_wait_timeout(1).expect("1 ms ok");
        check_finite_wait_timeout(DEFAULT_WAIT_CMD_TIMEOUT_MS).expect("default ok");
    }

    #[test]
    fn check_wait_timeout_infinite_opt_in_policy() {
        // Without ALLOW_INFINITE env, 0 is refused (same default as CI).
        // Do not mutate process env here if other tests race; document that
        // XDNA2_WAIT_ALLOW_INFINITE is opt-in for operators only.
        if !wait_infinite_is_allowed() {
            let err = check_wait_timeout_or_infinite_opt_in(0).expect_err("infinite closed");
            assert!(matches!(err, DirectError::InvalidArgument(_)));
            assert!(
                err.to_string().contains("XDNA2_WAIT_ALLOW_INFINITE")
                    || err.to_string().contains("infinite"),
                "err={err}"
            );
        }
        check_wait_timeout_or_infinite_opt_in(100).expect("finite always ok");
    }

    #[test]
    fn fill_cmd_bo_refuses_empty_payload() {
        let stub = StubDirectSubmission::new(-1);
        let err = fill_cmd_bo_with_payload(&stub, 0, 1, &[]).unwrap_err();
        assert!(matches!(err, DirectError::InvalidArgument(_)));
        assert!(
            err.to_string().contains("empty") || err.to_string().contains("wedge"),
            "err={err}"
        );
    }

    #[test]
    fn stub_fill_cmd_bo_accepts_nonempty_envelope() {
        let stub = StubDirectSubmission::new(-1);
        let payload = build_ert_start_npu_cmd_bo(0, 0, &[], 0x1).expect("envelope");
        fill_cmd_bo_with_payload(&stub, 0, 42, &payload).expect("stub fill");
    }

    #[test]
    fn stub_wait_command_succeeds_without_exec_cmd_gate() {
        // Wait is intentionally not EXEC_CMD-gated (completion/cleanup path).
        assert!(
            !exec_cmd_is_enabled(),
            "wait_command test assumes gated process for documentation clarity"
        );
        let stub = StubDirectSubmission::new(-1);
        stub.wait_command(0, 1, 0, 100)
            .expect("stub wait_command must succeed without EXEC_CMD enable");
    }

    #[test]
    fn real_wait_command_not_blocked_by_exec_cmd_gate_message() {
        // Gate must not short-circuit wait with exec_cmd_block_reason.
        // Fake fd: expect ioctl failure, not Unsupported(exec_cmd_block_reason).
        assert!(!exec_cmd_is_enabled());
        let real = RealDirectSubmission::new(42);
        let result = real.wait_command(0, 1, 99, 10);
        match result {
            Ok(()) | Err(DirectError::IoctlFailed(_)) => {
                // Ok: platform no-op on fake fd, or expected ioctl error after gate skipped.
            }
            Err(DirectError::Unsupported(msg)) => {
                assert!(
                    !msg.contains("EXEC_CMD disabled")
                        && !msg.contains("XDNA2_ALLOW_EXEC_CMD")
                        && msg != exec_cmd_block_reason(),
                    "wait must not use EXEC_CMD gate message: {msg}"
                );
            }
            Err(other) => {
                // DriverError / InvalidArgument also fine — just not the submit gate.
                let s = other.to_string();
                assert_ne!(s, exec_cmd_block_reason());
            }
        }
    }

    /// Hardware single-shot is operator-only. Never runs under default `cargo test`.
    ///
    /// Requires: live `/dev/accel`, L1 double gate open, L3 `exec_cmd_allow=1` if
    /// lab module is loaded, and env `XDNA2_HW_EXEC=1`. See
    /// `docs/XDNA2_EXEC_CMD_LAB.md`. Do **not** enable this in CI.
    #[test]
    #[ignore = "operator-only lab EXEC_CMD; set XDNA2_HW_EXEC=1 + double gate; see XDNA2_EXEC_CMD_LAB.md"]
    fn hw_exec_cmd_submit_and_wait_single_shot_ignored_by_default() {
        // Placeholder: ignored so `cargo test --lib` never hits EXEC_CMD on silicon.
        // Operators run: cargo test --features amd-xdna2,xdna2-dangerous-exec-cmd \
        //   --lib hw_exec_cmd_submit_and_wait -- --ignored
        // with XDNA2_ALLOW_EXEC_CMD=1 and XDNA2_HW_EXEC=1 after diagnose LIKELY_OK.
        if std::env::var("XDNA2_HW_EXEC").ok().as_deref() != Some("1") {
            return;
        }
        assert!(
            exec_cmd_is_enabled(),
            "XDNA2_HW_EXEC=1 requires feature xdna2-dangerous-exec-cmd + XDNA2_ALLOW_EXEC_CMD=1"
        );
        // Full single-shot is documented in docs/XDNA2_EXEC_CMD_LAB.md (open device,
        // create hwctx, build known-good payload, submit_and_wait once). Not auto-run.
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
        assert!(names.contains(&"bo_roundtrip"));
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
    /// Timed matmul / buffer op with structured timings (NPU GEMM Pass only if `ran_on_npu`).
    TimedGemm,
    /// Timed Shmem BO fill + SYNC TO/FROM + verify (safe DMA path; not NPU compute).
    BoRoundtrip,
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
            Self::BoRoundtrip => "bo_roundtrip",
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
            "bo_roundtrip",
        ]
    }
}
