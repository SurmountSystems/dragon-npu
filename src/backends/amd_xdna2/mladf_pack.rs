//! AMD `mladfmatmulbias` buffer packing (lab path).
//!
//! # Plain English
//!
//! The extracted Strix matmul **program** does not eat plain f32 ones. It wants:
//! - **IFM**: 16-bit activations (bf16-ish), row-major  
//! - **WTS**: heavily shuffled 4-bit weights + scales/bias (opaque shuffle for now)  
//! - **OFM**: 16-bit outputs  
//!
//! Exact sizes for the staged key come from AMD `*_ctrl_meta` JSON.
//!
//! # Scope
//!
//! - Parse instr keys → M/K/N/group_size  
//! - Size math + meta JSON reader  
//! - Pure IFM/OFM bf16 pack/unpack  
//! - WTS: load pre-packed file **or** zero-fill of correct length (not a fake shuffle)  
//! - No EXEC, no Pass claim  
//!
//! See `docs/AMD_MLADF_MATMUL_PACK.md`.

use super::direct::DirectError;
use serde_json::Value;

/// Element size for a16 / acc16f views (bf16 stored as `u16` LE).
pub const MLADF_A16_ELEM_BYTES: u64 = 2;

/// Canonical staged lab key (Strix flat a16w4).
pub const STAGED_MLADF_KEY: &str =
    "mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128";

/// Documented IFM size for [`STAGED_MLADF_KEY`] from AMD `*_ctrl_meta`.
pub const STAGED_MLADF_IFM_BYTES: u64 = 2304;
/// Documented WTS size for [`STAGED_MLADF_KEY`] from AMD `*_ctrl_meta`.
pub const STAGED_MLADF_WTS_BYTES: u64 = 174_080;
/// Documented OFM size for [`STAGED_MLADF_KEY`] from AMD `*_ctrl_meta`.
pub const STAGED_MLADF_OFM_BYTES: u64 = 512;
/// Documented control-packet size for [`STAGED_MLADF_KEY`] (EXEC follow-up).
pub const STAGED_MLADF_CTRL_BYTES: u64 = 29_664;

/// Cap for lab file reads (WTS / meta / golden) and meta `size_in_bytes` (64 MiB).
///
/// Matches [`super::direct::MAX_INSTRUCTION_BLOB_BYTES`] — refuse sparse/huge files
/// before full `read` / huge vector alloc.
pub const MAX_MLADF_LAB_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// Parsed shape from an AMD DynamicDispatch instr key suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MladfMatmulShape {
    /// Rows of A / C (batch token dim for LLM GEMM).
    pub m: u32,
    /// Inner / K.
    pub k: u32,
    /// Columns of B / C.
    pub n: u32,
    /// Quant group size (0 if absent in key).
    pub group_size: u32,
}

/// Optional dtype / layout tags parsed from a key (best-effort, non-fatal).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MladfKeyTags {
    /// Raw middle tokens (excluding op prefix and trailing M/K/N/Gs digits).
    pub tokens: Vec<String>,
    /// True if key suggests a16 / bf16 activations.
    pub a16: bool,
    /// True if key suggests w4 packed weights.
    pub w4: bool,
    /// True if key mentions acc16f / 16-bit accumulate/out.
    pub acc16f: bool,
}

impl MladfMatmulShape {
    /// IFM bytes for a16 (M·K·2).
    ///
    /// # Errors
    ///
    /// Zero dim or overflow.
    pub fn ifm_nbytes_a16(self) -> Result<u64, DirectError> {
        a16_matrix_nbytes(self.m, self.k, "IFM")
    }

    /// OFM bytes for a16/acc16 (M·N·2).
    ///
    /// # Errors
    ///
    /// Zero dim or overflow.
    pub fn ofm_nbytes_a16(self) -> Result<u64, DirectError> {
        a16_matrix_nbytes(self.m, self.n, "OFM")
    }
}

/// Full lab size plan for one mladf kernel (from key + optional meta).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MladfBufferPlan {
    /// Matmul shape.
    pub shape: MladfMatmulShape,
    /// IFM payload bytes.
    pub ifm_bytes: u64,
    /// WTS payload bytes (must match meta / packed file).
    pub wts_bytes: u64,
    /// OFM payload bytes.
    pub ofm_bytes: u64,
    /// Optional runtime control packet size (0 if unknown).
    pub ctrl_bytes: u64,
    /// Instr key that produced this plan (owned for logging).
    pub key: String,
}

/// Lab pack mode for IFM/WTS/OFM fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabPackMode {
    /// Historical default: f32 ones (Phase 3.5 ones golden).
    F32Ones,
    /// AMD mladf a16w4 sizes + bf16 IFM + packed/zero WTS.
    MladfA16W4,
}

impl LabPackMode {
    /// Parse `XDNA2_LAB_PACK` (`f32_ones` default, `mladf_a16w4`).
    ///
    /// **Fail-closed on unknown values** (typos do not silently become ones).
    /// Empty / unset → [`LabPackMode::F32Ones`].
    ///
    /// # Errors
    ///
    /// Unknown non-empty pack mode string.
    pub fn try_from_env_raw(raw: Option<&str>) -> Result<Self, DirectError> {
        let t = raw.map_or("", str::trim);
        if t.is_empty()
            || t.eq_ignore_ascii_case("f32_ones")
            || t.eq_ignore_ascii_case("f32")
            || t.eq_ignore_ascii_case("ones")
            || t.eq_ignore_ascii_case("default")
        {
            return Ok(Self::F32Ones);
        }
        if t.eq_ignore_ascii_case("mladf_a16w4")
            || t.eq_ignore_ascii_case("mladf")
            || t.eq_ignore_ascii_case("a16w4")
        {
            return Ok(Self::MladfA16W4);
        }
        Err(DirectError::InvalidArgument(format!(
            "XDNA2_LAB_PACK unknown value {t:?} (want f32_ones|mladf_a16w4 or unset)"
        )))
    }

    /// Read `XDNA2_LAB_PACK` from the process environment (fail-closed on typos).
    ///
    /// # Errors
    ///
    /// Unknown pack mode string.
    pub fn try_from_env() -> Result<Self, DirectError> {
        Self::try_from_env_raw(std::env::var("XDNA2_LAB_PACK").ok().as_deref())
    }
}

fn a16_matrix_nbytes(rows: u32, cols: u32, role: &str) -> Result<u64, DirectError> {
    if rows == 0 || cols == 0 {
        return Err(DirectError::InvalidArgument(format!(
            "mladf {role}: need positive dims (rows={rows} cols={cols})"
        )));
    }
    let elems = u64::from(rows)
        .checked_mul(u64::from(cols))
        .ok_or_else(|| {
            DirectError::InvalidArgument(format!("mladf {role}: element product overflow"))
        })?;
    elems
        .checked_mul(MLADF_A16_ELEM_BYTES)
        .ok_or_else(|| DirectError::InvalidArgument(format!("mladf {role}: byte size overflow")))
}

fn is_pure_digits(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

fn strip_file_extension(key: &str) -> String {
    let s = key.trim();
    if let Some(stripped) = s.strip_suffix(".json") {
        return stripped.to_string();
    }
    if let Some(stripped) = s.strip_suffix(".bin") {
        return stripped.to_string();
    }
    s.to_string()
}

/// Strip only map-sibling / dump suffixes (`_ctrl_meta`, `_ctrl`, `_param`, extensions).
///
/// Does **not** strip GM tails — use [`normalize_mladf_instr_key`] for full shape peel.
#[must_use]
pub fn strip_mladf_map_sibling_suffixes(key: &str) -> String {
    let mut s = strip_file_extension(key);
    loop {
        let before = s.clone();
        if let Some(x) = s.strip_suffix("_ctrl_meta") {
            s = x.to_string();
            continue;
        }
        if let Some(x) = s.strip_suffix("_ctrl") {
            s = x.to_string();
            continue;
        }
        if let Some(x) = s.strip_suffix("_param") {
            s = x.to_string();
            continue;
        }
        if s == before {
            break;
        }
    }
    s
}

/// True when `key` is the staged lab key (or its `_ctrl` / `_ctrl_meta` / file basename).
///
/// GM superstrings (`…_1_16384_gm`) are **false** even though they share the same
/// M/K/N/Gs after shape normalize — they must not inherit staged OFM/CTRL sizes.
#[must_use]
pub fn is_exact_staged_mladf_key(key: &str) -> bool {
    strip_mladf_map_sibling_suffixes(key) == STAGED_MLADF_KEY
}

/// Strip dump extensions / map siblings / GM suffixes so the key ends at `M_K_N(_Gs)`.
///
/// Handles filenames (`*.json` / `*.bin`), map siblings (`_ctrl_meta`, `_ctrl`, `_param`),
/// and GM tails (`_<a>_<b>_gm`, bare `_gm`).
#[must_use]
pub fn normalize_mladf_instr_key(key: &str) -> String {
    let mut s = strip_mladf_map_sibling_suffixes(key);
    // Peel GM tails until stable (`…_1_16384_gm` → base matmul key).
    loop {
        let before = s.clone();
        if let Some(without_gm) = s.strip_suffix("_gm") {
            let parts: Vec<&str> = without_gm.rsplitn(3, '_').collect();
            if parts.len() == 3 && is_pure_digits(parts[0]) && is_pure_digits(parts[1]) {
                s = parts[2].to_string();
            } else {
                s = without_gm.to_string();
            }
            continue;
        }
        if s == before {
            break;
        }
    }
    s
}

/// Parse trailing `…_M_K_N` or `…_M_K_N_Gs` from an AMD instr key.
///
/// Accepts full keys like
/// `mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128`
/// and GM variants such as `…_1_1152_256_128_1_16384_gm` (GM tail stripped first).
///
/// After normalization there must be **exactly** 3 or 4 trailing pure-digit groups
/// (fail-closed if extra digit groups remain).
///
/// # Errors
///
/// Missing / non-numeric trailing dims, extra trailing digit groups, or zero dims.
pub fn parse_mladf_instr_key(key: &str) -> Result<MladfMatmulShape, DirectError> {
    let key = key.trim();
    if key.is_empty() {
        return Err(DirectError::InvalidArgument(
            "mladf instr key is empty".into(),
        ));
    }
    let base = normalize_mladf_instr_key(key);
    if base.is_empty() {
        return Err(DirectError::InvalidArgument(
            "mladf instr key empty after suffix strip".into(),
        ));
    }
    let parts: Vec<&str> = base.split('_').collect();
    // Collect *all* trailing pure-digit tokens (not just first 4).
    let mut nums: Vec<u32> = Vec::new();
    for p in parts.iter().rev() {
        if is_pure_digits(p) {
            let v: u32 = p.parse().map_err(|_| {
                DirectError::InvalidArgument(format!("mladf key dim overflow in {p:?}"))
            })?;
            nums.push(v);
        } else if !nums.is_empty() {
            // Hit the non-digit token immediately before the shape tail.
            break;
        }
        // else: skip non-digit trailing garbage while still empty (should not happen).
    }
    // nums is reversed: trailing groups from the end.
    // Exactly 3 → M_K_N (Gs=0); exactly 4 → M_K_N_Gs. Anything else is fail-closed.
    match nums.as_slice() {
        [n, k, m] => shape_from_parts(*m, *k, *n, 0),
        [gs, n, k, m] => shape_from_parts(*m, *k, *n, *gs),
        _ => Err(DirectError::InvalidArgument(format!(
            "mladf instr key needs exactly trailing M_K_N or M_K_N_Gs digits after suffix \
             strip (got {count} trailing digit groups from {key:?}; base={base:?})",
            count = nums.len()
        ))),
    }
}

fn shape_from_parts(
    m: u32,
    k: u32,
    n: u32,
    group_size: u32,
) -> Result<MladfMatmulShape, DirectError> {
    if m == 0 || k == 0 || n == 0 {
        return Err(DirectError::InvalidArgument(format!(
            "mladf shape dims must be non-zero (M={m} K={k} N={n})"
        )));
    }
    Ok(MladfMatmulShape {
        m,
        k,
        n,
        group_size,
    })
}

/// Best-effort tag scrape from an instr key (dtype/layout tokens).
///
/// Does not fail on unknown keys; empty tags if nothing recognizable.
#[must_use]
pub fn parse_mladf_key_tags(key: &str) -> MladfKeyTags {
    let base = normalize_mladf_instr_key(key);
    let parts: Vec<&str> = base.split('_').collect();
    // Drop trailing pure-digit groups (M/K/N/Gs).
    let mut end = parts.len();
    while end > 0 {
        let p = parts[end - 1];
        if is_pure_digits(p) {
            end -= 1;
        } else {
            break;
        }
    }
    let mut tags = MladfKeyTags::default();
    for p in &parts[..end] {
        if p.is_empty() {
            continue;
        }
        tags.tokens.push((*p).to_string());
        // Composite tokens often pack everything: a16fw4acc16f
        let lower = p.to_ascii_lowercase();
        if lower.contains("a16") || lower.contains("bf16") {
            tags.a16 = true;
        }
        if lower.contains("w4") {
            tags.w4 = true;
        }
        if lower.contains("acc16") {
            tags.acc16f = true;
        }
    }
    tags
}

/// Known buffer plan for the staged Strix lab key (hard-coded meta sizes).
#[must_use]
pub fn staged_mladf_buffer_plan() -> MladfBufferPlan {
    MladfBufferPlan {
        shape: MladfMatmulShape {
            m: 1,
            k: 1152,
            n: 256,
            group_size: 128,
        },
        ifm_bytes: STAGED_MLADF_IFM_BYTES,
        wts_bytes: STAGED_MLADF_WTS_BYTES,
        ofm_bytes: STAGED_MLADF_OFM_BYTES,
        ctrl_bytes: STAGED_MLADF_CTRL_BYTES,
        key: STAGED_MLADF_KEY.to_string(),
    }
}

/// Build a lab buffer plan from an instr key (a16 IFM/OFM math + optional WTS).
///
/// WTS is built-in only when [`is_exact_staged_mladf_key`] is true (map-sibling /
/// extension strip only; **GM tails are never exact-staged**), or when
/// `wts_bytes_override` is provided.
///
/// Superstrings (e.g. `…_1_16384_gm`) share M/K/N/Gs after shape normalize but are
/// **not** treated as staged — they must **not** unlock staged OFM/CTRL constants.
///
/// For non-exact keys:
/// - `wts_bytes_override` alone fills WTS length only; OFM stays `M*N*2` and
///   `ctrl_bytes = 0` (a16 formula — wrong for GM kernels with padded OFM).
/// - Prefer [`parse_mladf_ctrl_meta_json`] (or lab env `XDNA2_LAB_MLADF_META`) for
///   authoritative IFM/WTS/OFM/ctrl BO sizes from AMD meta.
///
/// # Errors
///
/// Key parse failure, or missing WTS size for non-staged keys.
pub fn mladf_buffer_plan_from_key(
    key: &str,
    wts_bytes_override: Option<u64>,
) -> Result<MladfBufferPlan, DirectError> {
    let shape = parse_mladf_instr_key(key)?;
    let ifm_bytes = shape.ifm_nbytes_a16()?;
    let ofm_bytes = shape.ofm_nbytes_a16()?;
    // Exact staged key only (sibling/ext strip, **not** GM peel) — never `contains`.
    if is_exact_staged_mladf_key(key) {
        let mut p = staged_mladf_buffer_plan();
        // Prefer caller key string as stored.
        p.key = key.trim().to_string();
        if let Some(w) = wts_bytes_override {
            p.wts_bytes = w;
        }
        // Sanity: IFM/OFM from shape must match staged constants.
        if ifm_bytes != STAGED_MLADF_IFM_BYTES || ofm_bytes != STAGED_MLADF_OFM_BYTES {
            return Err(DirectError::InvalidArgument(format!(
                "staged mladf key shape/size mismatch (ifm {ifm_bytes} ofm {ofm_bytes})"
            )));
        }
        return Ok(p);
    }
    let wts_bytes = wts_bytes_override.ok_or_else(|| {
        DirectError::InvalidArgument(format!(
            "mladf key {key:?}: unknown WTS size — pass wts_bytes_override or ctrl_meta \
             (only exact staged key {STAGED_MLADF_KEY} has a built-in WTS size)"
        ))
    })?;
    Ok(MladfBufferPlan {
        shape,
        ifm_bytes,
        wts_bytes,
        ofm_bytes,
        ctrl_bytes: 0,
        key: key.trim().to_string(),
    })
}

/// Parse AMD `*_ctrl_meta` JSON for coalesced IFM/WTS/OFM(/ctrl) sizes.
///
/// Looks for `external_buffers.*.name` containing `input` / `weight` / `output` /
/// `control` (case-insensitive) and reads `size_in_bytes`.
///
/// **Meta is source of truth** for buffer sizes (including OFM/WTS/ctrl). When the
/// key tags as a16, IFM is still cross-checked against `M*K*2` (activation formula).
/// OFM is **not** forced to `M*N*2` — GM variants pad OFM far beyond the logical
/// matrix (e.g. 8 MiB) while keeping the same M/K/N.
///
/// # Errors
///
/// Invalid JSON, missing required IFM/WTS/OFM sizes, or a16 IFM size mismatch.
pub fn parse_mladf_ctrl_meta_json(
    json_text: &str,
    key: &str,
) -> Result<MladfBufferPlan, DirectError> {
    let shape = parse_mladf_instr_key(key)?;
    let tags = parse_mladf_key_tags(key);
    let v: Value = serde_json::from_str(json_text).map_err(|e| {
        DirectError::InvalidArgument(format!("mladf ctrl_meta JSON parse failed: {e}"))
    })?;
    let mut ifm = None;
    let mut wts = None;
    let mut ofm = None;
    let mut ctrl = 0_u64;
    if let Some(ext) = v.get("external_buffers").and_then(Value::as_object) {
        for (_k, buf) in ext {
            let name = buf
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            let size = buf
                .get("size_in_bytes")
                .and_then(Value::as_u64)
                .ok_or_else(|| {
                    DirectError::InvalidArgument(format!(
                        "mladf ctrl_meta buffer {name:?} missing size_in_bytes"
                    ))
                })?;
            if size > MAX_MLADF_LAB_FILE_BYTES {
                return Err(DirectError::InvalidArgument(format!(
                    "mladf ctrl_meta buffer {name:?} size_in_bytes={size} > max \
                     {MAX_MLADF_LAB_FILE_BYTES}"
                )));
            }
            if name.contains("input") {
                ifm = Some(size);
            } else if name.contains("weight") {
                wts = Some(size);
            } else if name.contains("output") {
                ofm = Some(size);
            } else if name.contains("control") {
                ctrl = size;
            }
        }
    }
    let ifm_bytes = ifm.ok_or_else(|| {
        DirectError::InvalidArgument("mladf ctrl_meta: missing coalesed_inputs size".into())
    })?;
    let wts_bytes = wts.ok_or_else(|| {
        DirectError::InvalidArgument("mladf ctrl_meta: missing coalesed_weights size".into())
    })?;
    let ofm_bytes = ofm.ok_or_else(|| {
        DirectError::InvalidArgument("mladf ctrl_meta: missing coalesed_outputs size".into())
    })?;
    // a16 IFM only: OFM/WTS/ctrl trust meta (GM pads OFM; other dtypes differ).
    if tags.a16 {
        let formula_ifm = shape.ifm_nbytes_a16()?;
        if ifm_bytes != formula_ifm {
            return Err(DirectError::InvalidArgument(format!(
                "mladf ctrl_meta IFM size {ifm_bytes} != a16 M*K*2={formula_ifm}"
            )));
        }
    }
    Ok(MladfBufferPlan {
        shape,
        ifm_bytes,
        wts_bytes,
        ofm_bytes,
        ctrl_bytes: ctrl,
        key: key.trim().to_string(),
    })
}

// ---------------------------------------------------------------------------
// Control-packet patch locations (from *_ctrl_meta JSON)
// ---------------------------------------------------------------------------

/// Which coalesced external buffer a ctrl-packet patch targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CtrlPatchBufferRole {
    /// IFM / `coalesed_inputs` / `ifm_ddr`.
    Ifm,
    /// WTS / `coalesed_weights` / `wts_ddr`.
    Wts,
    /// OFM / `coalesed_outputs` / `ofm_ddr`.
    Ofm,
    /// Control packet itself (rarely has self-patches).
    Ctrl,
    /// Unclassified buffer name.
    Other,
}

/// One meta `control_packet_patch_locations[]` entry.
///
/// Staged AMD meta uses `operation: "read_add_write"` and `size: 6`. Live apply
/// is [`super::lab_io::apply_ctrl_patch_location`]: read existing LE field, **add**
/// BO base paddr, write back `size` bytes (preserves tile sub-offsets like OFM
/// `0x100` / WTS `0x15400`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtrlPatchLocation {
    /// Byte offset into the control packet.
    pub offset: usize,
    /// Meta `size` field (often 6 = 48-bit LE address field).
    pub size: u32,
    /// Meta `operation` (e.g. `read_add_write`).
    pub operation: String,
}

/// Patches for one external / coalesced buffer role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtrlMetaPatchEntry {
    /// Buffer role (IFM/WTS/OFM/CTRL/Other).
    pub role: CtrlPatchBufferRole,
    /// Buffer `name` from meta (e.g. `coalesed_inputs`, `compute_graph.ifm_ddr`).
    pub buffer_name: String,
    /// Patch sites inside the control packet.
    pub locations: Vec<CtrlPatchLocation>,
}

fn ctrl_patch_role_from_name(name: &str) -> CtrlPatchBufferRole {
    let n = name.to_ascii_lowercase();
    if n.contains("input") || n.contains("ifm") {
        CtrlPatchBufferRole::Ifm
    } else if n.contains("weight") || n.contains("wts") {
        CtrlPatchBufferRole::Wts
    } else if n.contains("output") || n.contains("ofm") {
        CtrlPatchBufferRole::Ofm
    } else if n.contains("control") || n.contains("ctrl") {
        CtrlPatchBufferRole::Ctrl
    } else {
        CtrlPatchBufferRole::Other
    }
}

fn parse_patch_locations_array(arr: &Value) -> Result<Vec<CtrlPatchLocation>, DirectError> {
    let Some(items) = arr.as_array() else {
        return Err(DirectError::InvalidArgument(
            "mladf ctrl_meta control_packet_patch_locations must be a JSON array".into(),
        ));
    };
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let offset = item.get("offset").and_then(Value::as_u64).ok_or_else(|| {
            DirectError::InvalidArgument(
                "mladf ctrl_meta patch location missing numeric offset".into(),
            )
        })?;
        let offset = usize::try_from(offset).map_err(|_| {
            DirectError::InvalidArgument(format!(
                "mladf ctrl_meta patch offset {offset} does not fit usize"
            ))
        })?;
        let size_u64 = item.get("size").and_then(Value::as_u64).unwrap_or(8);
        let size = u32::try_from(size_u64).map_err(|_| {
            DirectError::InvalidArgument(format!(
                "mladf ctrl_meta patch size {size_u64} does not fit u32"
            ))
        })?;
        let operation = item
            .get("operation")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        out.push(CtrlPatchLocation {
            offset,
            size,
            operation,
        });
    }
    Ok(out)
}

/// Parse `control_packet_patch_locations` from AMD `*_ctrl_meta` JSON.
///
/// Walks `external_buffers.*.control_packet_patch_locations` and nested
/// `coalesed_buffers[].control_packet_patch_locations`. Does **not** apply
/// patches; use lab `patch_ctrl_from_meta_io_paddrs` (size=6 `read_add_write`).
///
/// # Errors
///
/// Invalid JSON, `control_packet_patch_locations` present but not an array, or
/// patch entries missing numeric `offset`.
pub fn parse_mladf_ctrl_patch_locations(
    json_text: &str,
) -> Result<Vec<CtrlMetaPatchEntry>, DirectError> {
    let v: Value = serde_json::from_str(json_text).map_err(|e| {
        DirectError::InvalidArgument(format!("mladf ctrl_meta JSON parse failed: {e}"))
    })?;
    let mut entries = Vec::new();
    let Some(ext) = v.get("external_buffers").and_then(Value::as_object) else {
        return Ok(entries);
    };
    for (_k, buf) in ext {
        let top_name = buf
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if let Some(locs) = buf.get("control_packet_patch_locations") {
            let locations = parse_patch_locations_array(locs)?;
            if !locations.is_empty() {
                entries.push(CtrlMetaPatchEntry {
                    role: ctrl_patch_role_from_name(&top_name),
                    buffer_name: top_name.clone(),
                    locations,
                });
            }
        }
        if let Some(coalesed) = buf.get("coalesed_buffers").and_then(Value::as_array) {
            for cb in coalesed {
                let cb_name = cb
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or(top_name.as_str())
                    .to_string();
                if let Some(locs) = cb.get("control_packet_patch_locations") {
                    let locations = parse_patch_locations_array(locs)?;
                    if !locations.is_empty() {
                        // Prefer leaf name for role (ifm_ddr / wts_ddr / ofm_ddr).
                        entries.push(CtrlMetaPatchEntry {
                            role: ctrl_patch_role_from_name(&cb_name),
                            buffer_name: cb_name,
                            locations,
                        });
                    }
                }
            }
        }
    }
    Ok(entries)
}

/// Expand meta patch entries into `(offset, paddr)` pairs for IFM/WTS/OFM.
///
/// Every location under an IFM/WTS/OFM role gets that role's paddr. `Ctrl` /
/// `Other` roles are skipped (no inventing paddrs). Pure; does not write bytes.
#[must_use]
pub fn ctrl_u64_patches_for_io_paddrs(
    entries: &[CtrlMetaPatchEntry],
    ifm_paddr: u64,
    wts_paddr: u64,
    ofm_paddr: u64,
) -> Vec<(usize, u64)> {
    let mut out = Vec::new();
    for e in entries {
        let paddr = match e.role {
            CtrlPatchBufferRole::Ifm => ifm_paddr,
            CtrlPatchBufferRole::Wts => wts_paddr,
            CtrlPatchBufferRole::Ofm => ofm_paddr,
            CtrlPatchBufferRole::Ctrl | CtrlPatchBufferRole::Other => continue,
        };
        for loc in &e.locations {
            out.push((loc.offset, paddr));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// bf16 (IEEE-like) pack / unpack — used for a16 IFM/OFM views
// ---------------------------------------------------------------------------

/// Convert f32 → bf16 bits (round-to-nearest-even, truncate mantissa).
#[must_use]
pub fn f32_to_bf16_bits(v: f32) -> u16 {
    let bits = v.to_bits();
    // Round: add bias based on truncated bits before shift.
    let rounding_bias = 0x0000_7fff_u32 + ((bits >> 16) & 1);
    ((bits.wrapping_add(rounding_bias)) >> 16) as u16
}

/// Convert bf16 bits → f32 (shift into high half).
#[must_use]
pub fn bf16_bits_to_f32(bits: u16) -> f32 {
    f32::from_bits(u32::from(bits) << 16)
}

/// Pack row-major f32 matrix as LE bf16 bytes (`rows * cols * 2`).
///
/// # Errors
///
/// Length mismatch or zero dims.
pub fn pack_ifm_bf16_from_f32_rowmajor(
    rows: u32,
    cols: u32,
    f32_rowmajor: &[f32],
) -> Result<Vec<u8>, DirectError> {
    let nbytes = a16_matrix_nbytes(rows, cols, "IFM")?;
    let need = (nbytes / MLADF_A16_ELEM_BYTES) as usize;
    if f32_rowmajor.len() != need {
        return Err(DirectError::InvalidArgument(format!(
            "pack_ifm_bf16: f32 len {} != rows*cols={}",
            f32_rowmajor.len(),
            need
        )));
    }
    let mut out = Vec::with_capacity(need.saturating_mul(2));
    for &v in f32_rowmajor {
        out.extend_from_slice(&f32_to_bf16_bits(v).to_le_bytes());
    }
    Ok(out)
}

/// All-ones bf16 IFM (`rows * cols` ones as LE bf16).
///
/// # Errors
///
/// Zero dims / overflow.
pub fn host_ones_bf16_matrix_bytes(rows: u32, cols: u32) -> Result<Vec<u8>, DirectError> {
    let n = (a16_matrix_nbytes(rows, cols, "ones")? / MLADF_A16_ELEM_BYTES) as usize;
    pack_ifm_bf16_from_f32_rowmajor(rows, cols, &vec![1.0_f32; n])
}

/// Unpack LE bf16 OFM bytes → f32 vector.
///
/// # Errors
///
/// Odd length.
pub fn unpack_ofm_bf16_to_f32(bytes: &[u8]) -> Result<Vec<f32>, DirectError> {
    if !bytes.len().is_multiple_of(2) {
        return Err(DirectError::InvalidArgument(format!(
            "unpack_ofm_bf16: length {} is not a multiple of 2",
            bytes.len()
        )));
    }
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        let bits = u16::from_le_bytes([chunk[0], chunk[1]]);
        out.push(bf16_bits_to_f32(bits));
    }
    Ok(out)
}

/// Host payloads for lab IFM / WTS / OFM fill (no device).
pub type MladfLabPayloads = (Vec<u8>, Vec<u8>, Vec<u8>);

/// Build IFM/WTS/OFM host payloads for a [`MladfBufferPlan`].
///
/// - IFM: bf16 ones (or `ifm_f32` if provided, length M·K)  
/// - WTS: `wts_packed` if provided (must match `wts_bytes`), else **zeros** of that length  
/// - OFM: zeros of `ofm_bytes` (device writes here)
///
/// # Errors
///
/// Size mismatches.
pub fn build_mladf_lab_payloads(
    plan: &MladfBufferPlan,
    ifm_f32: Option<&[f32]>,
    wts_packed: Option<&[u8]>,
) -> Result<MladfLabPayloads, DirectError> {
    let ifm = if let Some(v) = ifm_f32 {
        pack_ifm_bf16_from_f32_rowmajor(plan.shape.m, plan.shape.k, v)?
    } else {
        host_ones_bf16_matrix_bytes(plan.shape.m, plan.shape.k)?
    };
    if ifm.len() as u64 != plan.ifm_bytes {
        return Err(DirectError::InvalidArgument(format!(
            "mladf IFM packed {} != plan {}",
            ifm.len(),
            plan.ifm_bytes
        )));
    }
    let wts = if let Some(w) = wts_packed {
        if w.len() as u64 != plan.wts_bytes {
            return Err(DirectError::InvalidArgument(format!(
                "mladf WTS packed {} != plan {}",
                w.len(),
                plan.wts_bytes
            )));
        }
        w.to_vec()
    } else {
        let n = usize::try_from(plan.wts_bytes).map_err(|_| {
            DirectError::InvalidArgument("mladf WTS plan size exceeds usize".into())
        })?;
        vec![0_u8; n]
    };
    let ofm_n = usize::try_from(plan.ofm_bytes)
        .map_err(|_| DirectError::InvalidArgument("mladf OFM plan size exceeds usize".into()))?;
    let ofm = vec![0_u8; ofm_n];
    Ok((ifm, wts, ofm))
}

/// Read a lab file with metadata length cap (before full read).
///
/// # Errors
///
/// Missing/empty/oversize file or I/O failure.
pub fn read_lab_file_capped(path: &str, role: &str) -> Result<Vec<u8>, DirectError> {
    let path = path.trim();
    if path.is_empty() {
        return Err(DirectError::InvalidArgument(format!(
            "{role} path is empty"
        )));
    }
    let meta = std::fs::metadata(path).map_err(|e| {
        DirectError::InvalidArgument(format!("{role} metadata failed ({path}): {e}"))
    })?;
    let len = meta.len();
    if len == 0 {
        return Err(DirectError::InvalidArgument(format!(
            "{role} file is empty: {path}"
        )));
    }
    if len > MAX_MLADF_LAB_FILE_BYTES {
        return Err(DirectError::InvalidArgument(format!(
            "{role} file is {len} bytes > max {MAX_MLADF_LAB_FILE_BYTES}: {path}"
        )));
    }
    let bytes = std::fs::read(path)
        .map_err(|e| DirectError::InvalidArgument(format!("{role} read failed ({path}): {e}")))?;
    if bytes.is_empty() {
        return Err(DirectError::InvalidArgument(format!(
            "{role} file is empty: {path}"
        )));
    }
    if (bytes.len() as u64) > MAX_MLADF_LAB_FILE_BYTES {
        return Err(DirectError::InvalidArgument(format!(
            "{role} read size {} > max {MAX_MLADF_LAB_FILE_BYTES}: {path}",
            bytes.len()
        )));
    }
    Ok(bytes)
}

/// Load optional pre-packed WTS from `XDNA2_LAB_WTS_PACKED` path.
///
/// When `expected_len` is `Some`, the file must match that length exactly
/// (fail-closed before BO fill). Does **not** refuse all-zero (use
/// [`super::lab_io::validate_mladf_wts_packed_for_pass`] or
/// `XDNA2_LAB_REQUIRE_NONZERO_WTS=1` at fill).
///
/// Obtain real AMD packs via `tools/export_mladf_wts.sh` (DD `export_const_params`)
/// or operator capture — never a fake pure-Rust shuffle.
///
/// # Errors
///
/// I/O, empty, oversize, or length mismatch when the env var is set.
pub fn load_wts_packed_from_env(expected_len: Option<u64>) -> Result<Option<Vec<u8>>, DirectError> {
    let Ok(path) = std::env::var("XDNA2_LAB_WTS_PACKED") else {
        return Ok(None);
    };
    let path = path.trim();
    if path.is_empty() {
        return Err(DirectError::InvalidArgument(
            "XDNA2_LAB_WTS_PACKED is set but empty".into(),
        ));
    }
    let bytes = read_lab_file_capped(path, "XDNA2_LAB_WTS_PACKED")?;
    if let Some(want) = expected_len {
        if bytes.len() as u64 != want {
            return Err(DirectError::InvalidArgument(format!(
                "XDNA2_LAB_WTS_PACKED len {} != plan/expected {want} (path={path})",
                bytes.len()
            )));
        }
    }
    Ok(Some(bytes))
}

/// True if every byte is zero (empty → false). Shared with lab_io zero gates.
#[must_use]
pub fn wts_bytes_are_all_zero(bytes: &[u8]) -> bool {
    !bytes.is_empty() && bytes.iter().all(|&b| b == 0)
}

/// Pure offline gate: length matches and payload is not all zeros.
///
/// Same contract as shell `tools/validate_mladf_wts.sh` and lab Pass readiness.
/// Does **not** prove AMD shuffle correctness — only refuses obvious zero packs.
///
/// # Errors
///
/// Wrong length, empty, or all-zero.
pub fn validate_wts_packed_len_nonzero(wts: &[u8], expected_len: u64) -> Result<(), DirectError> {
    let got = wts.len() as u64;
    if got != expected_len {
        return Err(DirectError::InvalidArgument(format!(
            "mladf WTS len {got} != expected {expected_len} (staged a16w4 is 174080)"
        )));
    }
    if wts.is_empty() {
        return Err(DirectError::InvalidArgument("mladf WTS is empty".into()));
    }
    if wts_bytes_are_all_zero(wts) {
        return Err(DirectError::InvalidArgument(
            "mladf WTS is all zeros — not AMD-packed (use tools/export_mladf_wts.sh \
             / export_const_params / --wts-from)"
                .into(),
        ));
    }
    Ok(())
}

/// Resolve lab plan from env: `XDNA2_LAB_MLADF_KEY` or staged default.
///
/// Optional `XDNA2_LAB_MLADF_META=/path/to/ctrl_meta.json` for sizes.
///
/// # Errors
///
/// Key/meta parse failures.
pub fn mladf_buffer_plan_from_env() -> Result<MladfBufferPlan, DirectError> {
    let key = std::env::var("XDNA2_LAB_MLADF_KEY").unwrap_or_else(|_| STAGED_MLADF_KEY.to_string());
    if let Ok(meta_path) = std::env::var("XDNA2_LAB_MLADF_META") {
        let bytes = read_lab_file_capped(meta_path.trim(), "XDNA2_LAB_MLADF_META")?;
        let text = String::from_utf8(bytes).map_err(|e| {
            DirectError::InvalidArgument(format!(
                "XDNA2_LAB_MLADF_META is not UTF-8 ({}): {e}",
                meta_path.trim()
            ))
        })?;
        return parse_mladf_ctrl_meta_json(&text, key.trim());
    }
    mladf_buffer_plan_from_key(key.trim(), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_staged_key() {
        let s = parse_mladf_instr_key(STAGED_MLADF_KEY).expect("parse");
        assert_eq!(
            s,
            MladfMatmulShape {
                m: 1,
                k: 1152,
                n: 256,
                group_size: 128
            }
        );
        assert_eq!(s.ifm_nbytes_a16().unwrap(), STAGED_MLADF_IFM_BYTES);
        assert_eq!(s.ofm_nbytes_a16().unwrap(), STAGED_MLADF_OFM_BYTES);
        // WP0 acceptance: print IFM/OFM/WTS from meta-backed staged plan.
        let plan = staged_mladf_buffer_plan();
        assert_eq!(plan.ifm_bytes, 2304);
        assert_eq!(plan.ofm_bytes, 512);
        assert_eq!(plan.wts_bytes, 174_080);
        let tags = parse_mladf_key_tags(STAGED_MLADF_KEY);
        assert!(tags.a16, "staged key should tag a16");
        assert!(tags.w4, "staged key should tag w4");
        assert!(tags.acc16f, "staged key should tag acc16f");
    }

    #[test]
    fn parse_key_strips_ctrl_suffix() {
        let k = format!("{STAGED_MLADF_KEY}_ctrl_meta");
        let s = parse_mladf_instr_key(&k).unwrap();
        assert_eq!(s.m, 1);
        assert_eq!(s.group_size, 128);
        // Also `_ctrl` sibling (not only `_ctrl_meta`).
        let k2 = format!("{STAGED_MLADF_KEY}_ctrl");
        let s2 = parse_mladf_instr_key(&k2).unwrap();
        assert_eq!(s2, s);
        let tags = parse_mladf_key_tags(&k2);
        assert!(tags.a16 && tags.w4);
    }

    #[test]
    fn parse_key_strips_gm_and_extensions() {
        // Live dump sibling of staged key (was mis-parsed as M=256… before fix).
        let gm = format!("{STAGED_MLADF_KEY}_1_16384_gm");
        let s = parse_mladf_instr_key(&gm).expect("gm key");
        assert_eq!(
            s,
            MladfMatmulShape {
                m: 1,
                k: 1152,
                n: 256,
                group_size: 128
            }
        );
        // Filename footgun: paste dump basename into KEY env.
        let as_file = format!("{STAGED_MLADF_KEY}_ctrl_meta.json");
        assert_eq!(parse_mladf_instr_key(&as_file).unwrap().k, 1152);
        assert_eq!(
            normalize_mladf_instr_key(&format!("{STAGED_MLADF_KEY}.bin")),
            STAGED_MLADF_KEY
        );
    }

    #[test]
    fn parse_key_rejects_bad_inputs() {
        assert!(parse_mladf_instr_key("").is_err());
        assert!(parse_mladf_instr_key("not_a_key").is_err());
        assert!(parse_mladf_instr_key("only_two_1_2").is_err());
        // Zero dims.
        assert!(
            parse_mladf_instr_key(
                "mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_0_1152_256_128"
            )
            .is_err()
        );
        // Extra trailing digit groups without a known GM/suffix peel → fail-closed.
        let extra = format!("{STAGED_MLADF_KEY}_9_9_9");
        assert!(parse_mladf_instr_key(&extra).is_err());
    }

    #[test]
    fn staged_plan_exact_key_only() {
        let p = staged_mladf_buffer_plan();
        assert_eq!(p.ifm_bytes, 2304);
        assert_eq!(p.wts_bytes, 174_080);
        assert_eq!(p.ofm_bytes, 512);
        assert_eq!(p.ctrl_bytes, 29_664);
        // Exact (and suffix-stripped) staged key → built-in WTS.
        let from_exact = mladf_buffer_plan_from_key(STAGED_MLADF_KEY, None).unwrap();
        assert_eq!(from_exact.wts_bytes, STAGED_MLADF_WTS_BYTES);
        let from_ctrl =
            mladf_buffer_plan_from_key(&format!("{STAGED_MLADF_KEY}_ctrl_meta"), None).unwrap();
        assert_eq!(from_ctrl.wts_bytes, STAGED_MLADF_WTS_BYTES);
        // GM superstring must NOT inherit staged OFM/CTRL constants without meta.
        let gm = format!("{STAGED_MLADF_KEY}_1_16384_gm");
        assert!(
            mladf_buffer_plan_from_key(&gm, None).is_err(),
            "GM key must require wts override or ctrl_meta"
        );
        let gm_plan = mladf_buffer_plan_from_key(&gm, Some(174_080)).unwrap();
        assert_eq!(gm_plan.shape.m, 1);
        assert_eq!(gm_plan.ofm_bytes, 512); // a16 formula only; real GM OFM from meta
        assert_eq!(gm_plan.ctrl_bytes, 0);
    }

    #[test]
    fn parse_ctrl_meta_fail_closed() {
        assert!(parse_mladf_ctrl_meta_json("not-json", STAGED_MLADF_KEY).is_err());
        let missing_wts = r#"{
          "external_buffers": {
            "buffer0": { "name": "coalesed_inputs", "size_in_bytes": 2304 },
            "buffer1": { "name": "coalesed_outputs", "size_in_bytes": 512 }
          }
        }"#;
        assert!(parse_mladf_ctrl_meta_json(missing_wts, STAGED_MLADF_KEY).is_err());
        // a16 IFM must match M*K*2.
        let bad_ifm = r#"{
          "external_buffers": {
            "buffer0": { "name": "coalesed_inputs", "size_in_bytes": 999 },
            "buffer1": { "name": "coalesed_outputs", "size_in_bytes": 512 },
            "buffer2": { "name": "coalesed_weights", "size_in_bytes": 174080 }
          }
        }"#;
        assert!(parse_mladf_ctrl_meta_json(bad_ifm, STAGED_MLADF_KEY).is_err());
        // GM-style padded OFM is accepted (meta authoritative for OFM).
        let gm_key = format!("{STAGED_MLADF_KEY}_1_16384_gm");
        let gm_meta = r#"{
          "external_buffers": {
            "buffer0": { "name": "coalesed_inputs", "size_in_bytes": 2304 },
            "buffer1": { "name": "coalesed_outputs", "size_in_bytes": 8388608 },
            "buffer2": { "name": "coalesed_weights", "size_in_bytes": 174080 },
            "buffer3": { "name": "runtime_control_packet", "size_in_bytes": 29184 }
          }
        }"#;
        let p = parse_mladf_ctrl_meta_json(gm_meta, &gm_key).unwrap();
        assert_eq!(p.ofm_bytes, 8_388_608);
        assert_eq!(p.ctrl_bytes, 29_184);
        assert_eq!(p.shape.n, 256);
    }

    #[test]
    fn build_payloads_wts_len_mismatch() {
        let plan = staged_mladf_buffer_plan();
        let bad = vec![0_u8; 16];
        assert!(build_mladf_lab_payloads(&plan, None, Some(&bad)).is_err());
    }

    #[test]
    fn staged_plan_matches_constants() {
        let p = staged_mladf_buffer_plan();
        assert_eq!(p.ifm_bytes, 2304);
        assert_eq!(p.wts_bytes, 174_080);
        assert_eq!(p.ofm_bytes, 512);
        assert_eq!(p.ctrl_bytes, 29_664);
    }

    #[test]
    fn bf16_roundtrip_ones() {
        let bits = f32_to_bf16_bits(1.0);
        let back = bf16_bits_to_f32(bits);
        assert!((back - 1.0).abs() < 1e-2);
    }

    #[test]
    fn pack_ifm_ones_size() {
        let b = host_ones_bf16_matrix_bytes(1, 1152).unwrap();
        assert_eq!(b.len(), 2304);
        // bf16 1.0 is 0x3f80
        assert_eq!(&b[0..2], &0x3f80_u16.to_le_bytes());
    }

    #[test]
    fn build_payloads_zeros_wts() {
        let plan = staged_mladf_buffer_plan();
        let (ifm, wts, ofm) = build_mladf_lab_payloads(&plan, None, None).unwrap();
        assert_eq!(ifm.len() as u64, plan.ifm_bytes);
        assert_eq!(wts.len() as u64, plan.wts_bytes);
        assert_eq!(ofm.len() as u64, plan.ofm_bytes);
        assert!(wts.iter().all(|&x| x == 0));
        assert!(ofm.iter().all(|&x| x == 0));
    }

    #[test]
    fn parse_ctrl_meta_json_staged_shape() {
        let json = r#"{
          "external_buffers": {
            "buffer0": { "name": "coalesed_inputs", "size_in_bytes": 2304 },
            "buffer1": { "name": "coalesed_outputs", "size_in_bytes": 512 },
            "buffer2": { "name": "coalesed_weights", "size_in_bytes": 174080 },
            "buffer3": { "name": "runtime_control_packet", "size_in_bytes": 29664 }
          }
        }"#;
        let p = parse_mladf_ctrl_meta_json(json, STAGED_MLADF_KEY).unwrap();
        assert_eq!(p.ifm_bytes, 2304);
        assert_eq!(p.wts_bytes, 174_080);
        assert_eq!(p.ofm_bytes, 512);
        assert_eq!(p.ctrl_bytes, 29_664);
    }

    #[test]
    fn parse_ctrl_patch_locations_and_u64_list() {
        let json = r#"{
          "external_buffers": {
            "buffer0": {
              "name": "coalesed_inputs",
              "size_in_bytes": 2304,
              "coalesed_buffers": [{
                "name": "compute_graph.ifm_ddr",
                "offset_in_bytes": 0,
                "control_packet_patch_locations": [
                  {"offset": 7596, "operation": "read_add_write", "size": 6},
                  {"offset": 8976, "operation": "read_add_write", "size": 6}
                ]
              }]
            },
            "buffer1": {
              "name": "coalesed_outputs",
              "size_in_bytes": 512,
              "coalesed_buffers": [{
                "name": "compute_graph.ofm_ddr",
                "control_packet_patch_locations": [
                  {"offset": 7716, "operation": "read_add_write", "size": 6}
                ]
              }]
            },
            "buffer2": {
              "name": "coalesed_weights",
              "size_in_bytes": 174080,
              "coalesed_buffers": [{
                "name": "compute_graph.wts_ddr",
                "control_packet_patch_locations": [
                  {"offset": 7656, "operation": "read_add_write", "size": 6}
                ]
              }]
            },
            "buffer3": {
              "name": "runtime_control_packet",
              "size_in_bytes": 29664
            }
          }
        }"#;
        let entries = parse_mladf_ctrl_patch_locations(json).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].role, CtrlPatchBufferRole::Ifm);
        assert_eq!(entries[0].locations.len(), 2);
        assert_eq!(entries[0].locations[0].offset, 7596);
        assert_eq!(entries[0].locations[0].size, 6);
        assert_eq!(entries[0].locations[0].operation, "read_add_write");
        assert_eq!(entries[1].role, CtrlPatchBufferRole::Ofm);
        assert_eq!(entries[2].role, CtrlPatchBufferRole::Wts);
        let patches = ctrl_u64_patches_for_io_paddrs(&entries, 0x11, 0x22, 0x33);
        assert_eq!(patches.len(), 4); // 2 IFM + 1 OFM + 1 WTS
        assert!(patches.contains(&(7596, 0x11)));
        assert!(patches.contains(&(8976, 0x11)));
        assert!(patches.contains(&(7656, 0x22)));
        assert!(patches.contains(&(7716, 0x33)));
        // Fail-closed: missing offset
        let bad = r#"{"external_buffers":{"b":{"name":"coalesed_inputs",
          "coalesed_buffers":[{"name":"ifm","control_packet_patch_locations":[{"size":6}]}]}}}"#;
        assert!(parse_mladf_ctrl_patch_locations(bad).is_err());
        assert!(parse_mladf_ctrl_patch_locations("not-json").is_err());
        assert!(parse_mladf_ctrl_patch_locations("{}").unwrap().is_empty());
        // Present but not an array → refuse (no silent drop)
        let not_arr = r#"{"external_buffers":{"b":{"name":"coalesed_inputs",
          "control_packet_patch_locations":{"offset":1}}}}"#;
        let err = parse_mladf_ctrl_patch_locations(not_arr).expect_err("not array");
        assert!(err.to_string().contains("array"), "err={err}");
        let not_arr_nested = r#"{"external_buffers":{"b":{"name":"coalesed_inputs",
          "coalesed_buffers":[{"name":"ifm","control_packet_patch_locations":"nope"}]}}}"#;
        assert!(
            parse_mladf_ctrl_patch_locations(not_arr_nested)
                .unwrap_err()
                .to_string()
                .contains("array")
        );
        // Empty array is fine (no locations)
        let empty_arr = r#"{"external_buffers":{"b":{"name":"coalesed_inputs",
          "control_packet_patch_locations":[]}}}"#;
        assert!(
            parse_mladf_ctrl_patch_locations(empty_arr)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn lab_pack_mode_parse() {
        assert_eq!(
            LabPackMode::try_from_env_raw(Some("mladf_a16w4")).unwrap(),
            LabPackMode::MladfA16W4
        );
        assert_eq!(
            LabPackMode::try_from_env_raw(Some("mladf")).unwrap(),
            LabPackMode::MladfA16W4
        );
        assert_eq!(
            LabPackMode::try_from_env_raw(Some("a16w4")).unwrap(),
            LabPackMode::MladfA16W4
        );
        assert_eq!(
            LabPackMode::try_from_env_raw(None).unwrap(),
            LabPackMode::F32Ones
        );
        assert_eq!(
            LabPackMode::try_from_env_raw(Some("f32_ones")).unwrap(),
            LabPackMode::F32Ones
        );
        let err = LabPackMode::try_from_env_raw(Some("nope")).expect_err("unknown");
        assert!(err.to_string().contains("unknown"));
    }

    #[test]
    fn is_exact_staged_key_table() {
        assert!(is_exact_staged_mladf_key(STAGED_MLADF_KEY));
        assert!(is_exact_staged_mladf_key(&format!(
            "{STAGED_MLADF_KEY}_ctrl_meta"
        )));
        assert!(is_exact_staged_mladf_key(&format!(
            "{STAGED_MLADF_KEY}.json"
        )));
        assert!(!is_exact_staged_mladf_key(&format!(
            "{STAGED_MLADF_KEY}_1_16384_gm"
        )));
        assert!(!is_exact_staged_mladf_key("not_a_key"));
    }

    #[test]
    fn pack_ifm_from_non_ones_f32() {
        let f = [2.0_f32, 0.0, -1.0];
        let b = pack_ifm_bf16_from_f32_rowmajor(1, 3, &f).unwrap();
        assert_eq!(b.len(), 6);
        assert_eq!(&b[0..2], &f32_to_bf16_bits(2.0).to_le_bytes());
    }

    #[test]
    fn read_lab_file_capped_empty_and_ok() {
        let dir =
            std::env::temp_dir().join(format!("dragon_npu_lab_cap_{}_{}", std::process::id(), "t"));
        let _ = std::fs::create_dir_all(&dir);
        let empty = dir.join("empty.bin");
        std::fs::write(&empty, []).unwrap();
        let err = read_lab_file_capped(empty.to_str().unwrap(), "test").expect_err("empty");
        assert!(err.to_string().contains("empty"));
        let okp = dir.join("ok.bin");
        std::fs::write(&okp, [1_u8, 2, 3]).unwrap();
        let b = read_lab_file_capped(okp.to_str().unwrap(), "test").unwrap();
        assert_eq!(b, vec![1, 2, 3]);
        let _ = std::fs::remove_file(&empty);
        let _ = std::fs::remove_file(&okp);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    #[allow(unsafe_code)]
    fn load_wts_packed_expected_len_mismatch() {
        let dir = std::env::temp_dir().join(format!("dragon_npu_wts_len_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("wts.bin");
        std::fs::write(&path, [0_u8; 16]).unwrap();
        let prev = std::env::var_os("XDNA2_LAB_WTS_PACKED");
        // SAFETY: test-only WTS path.
        unsafe {
            std::env::set_var("XDNA2_LAB_WTS_PACKED", path.to_str().unwrap());
        }
        let err = load_wts_packed_from_env(Some(STAGED_MLADF_WTS_BYTES)).expect_err("len");
        assert!(err.to_string().contains("!=") || err.to_string().contains("expected"));
        // unset → None
        unsafe {
            std::env::remove_var("XDNA2_LAB_WTS_PACKED");
        }
        assert!(load_wts_packed_from_env(None).unwrap().is_none());
        unsafe {
            match prev {
                Some(v) => std::env::set_var("XDNA2_LAB_WTS_PACKED", v),
                None => std::env::remove_var("XDNA2_LAB_WTS_PACKED"),
            }
        }
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn staged_sizes_match_pack_tool_contract() {
        // Keep in lockstep with tools/pack_mladf_ref.cpp + export_mladf_wts.cpp.
        assert_eq!(STAGED_MLADF_IFM_BYTES, 2304);
        assert_eq!(STAGED_MLADF_WTS_BYTES, 174_080);
        assert_eq!(STAGED_MLADF_OFM_BYTES, 512);
        assert_eq!(STAGED_MLADF_CTRL_BYTES, 29_664);
        assert_eq!(MAX_MLADF_LAB_FILE_BYTES, 64 * 1024 * 1024);
    }

    #[test]
    fn validate_wts_packed_len_nonzero_staged() {
        let mut v = vec![0_u8; STAGED_MLADF_WTS_BYTES as usize];
        assert!(
            validate_wts_packed_len_nonzero(&v, STAGED_MLADF_WTS_BYTES)
                .unwrap_err()
                .to_string()
                .contains("all zeros")
        );
        v[100] = 0xAB;
        validate_wts_packed_len_nonzero(&v, STAGED_MLADF_WTS_BYTES).unwrap();
        assert!(
            validate_wts_packed_len_nonzero(&v[..16], STAGED_MLADF_WTS_BYTES)
                .unwrap_err()
                .to_string()
                .contains("len")
        );
        assert!(!wts_bytes_are_all_zero(&[]));
        assert!(wts_bytes_are_all_zero(&[0, 0]));
        assert!(!wts_bytes_are_all_zero(&[0, 1]));
    }

    #[test]
    fn ctrl_meta_refuses_huge_size_in_bytes() {
        let json = format!(
            r#"{{"external_buffers":{{"a":{{"name":"coalesed_inputs","size_in_bytes":2304}},
            "b":{{"name":"coalesed_weights","size_in_bytes":{}}},
            "c":{{"name":"coalesed_outputs","size_in_bytes":512}}}}}}"#,
            MAX_MLADF_LAB_FILE_BYTES + 1
        );
        let err = parse_mladf_ctrl_meta_json(&json, STAGED_MLADF_KEY).expect_err("huge wts");
        assert!(err.to_string().contains("max"));
    }

    #[test]
    fn unpack_ofm_len() {
        let bits = f32_to_bf16_bits(2.0).to_le_bytes();
        let v = unpack_ofm_bf16_to_f32(&bits).unwrap();
        assert_eq!(v.len(), 1);
        assert!((v[0] - 2.0).abs() < 0.05);
    }
}
