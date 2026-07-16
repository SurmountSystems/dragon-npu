//! Write a **research-only** classic v0.1 NOOP XAie txn to a path.
//!
//! ```text
//! cargo run --features amd-xdna2 --example materialize_research_txn -- testdata/xdna2_cmd/lab_instr.bin
//! ```
//!
//! **Not** a known-good GEMM. Valid header + op walk only.

#[cfg(feature = "amd-xdna2")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use dragon_npu::backends::amd_xdna2::{
        build_strix_research_noop_txn, classify_instruction_blob,
    };
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "testdata/xdna2_cmd/lab_instr.bin".to_string());
    if let Some(parent) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = build_strix_research_noop_txn()?;
    let kind = classify_instruction_blob(&bytes);
    std::fs::write(&path, &bytes)?;
    println!("wrote {} bytes to {path}", bytes.len());
    println!("classify: {}", kind.summary());
    println!("NOT_AIE2_KNOWN_GOOD_GEMM=1 (NOOP research txn only)");
    Ok(())
}

#[cfg(not(feature = "amd-xdna2"))]
fn main() {
    eprintln!("enable --features amd-xdna2");
    std::process::exit(2);
}
