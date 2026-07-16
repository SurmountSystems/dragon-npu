// pack_mladf_ref — offline host buffers for staged AMD mladf a16w4 lab key.
//
// Lab only. Writes correct *sizes* and a correct IFM bf16 pack.
// WTS is either:
//   - zero-filled at the meta length (documented partial — NOT an AMD shuffle), or
//   - copied from --wts-from / env if the operator already has AMD-packed bytes.
//
// We deliberately do **not** invent a fake weight shuffle and claim it is AMD.
// Linking full DynamicDispatch export_const_params needs XRT + heavy deps; this
// tool mirrors dump_dd_txn's light footprint (no XRT).
//
// Build + run: tools/pack_mladf_ref.sh
//
// Staged sizes MUST match Rust constants in src/backends/amd_xdna2/mladf_pack.rs:
//   STAGED_MLADF_IFM_BYTES=2304  STAGED_MLADF_WTS_BYTES=174080
//   STAGED_MLADF_OFM_BYTES=512   STAGED_MLADF_CTRL_BYTES=29664
// (cargo test: staged_sizes_match_pack_tool_contract)

#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

namespace fs = std::filesystem;

// Staged lab key (must match Rust STAGED_MLADF_*).
static constexpr const char *kStagedKey =
    "mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128";
static constexpr uint32_t kM = 1;
static constexpr uint32_t kK = 1152;
static constexpr uint32_t kN = 256;
static constexpr uint32_t kGs = 128;
static constexpr size_t kIfmBytes = 2304; // M*K*2  == STAGED_MLADF_IFM_BYTES
static constexpr size_t kWtsBytes = 174080; // == STAGED_MLADF_WTS_BYTES
static constexpr size_t kOfmBytes = 512;  // M*N*2  == STAGED_MLADF_OFM_BYTES
static constexpr size_t kCtrlBytes = 29664; // == STAGED_MLADF_CTRL_BYTES

static uint16_t f32_to_bf16_bits(float v) {
  uint32_t bits = 0;
  static_assert(sizeof(float) == 4, "float must be 32-bit");
  std::memcpy(&bits, &v, sizeof(bits));
  // Round-to-nearest-even bias (matches Rust f32_to_bf16_bits).
  const uint32_t rounding_bias = 0x00007fffu + ((bits >> 16) & 1u);
  return static_cast<uint16_t>((bits + rounding_bias) >> 16);
}

static void write_bytes(const fs::path &path, const std::vector<std::uint8_t> &v) {
  std::ofstream f(path, std::ios::binary);
  if (!f) {
    throw std::runtime_error("open failed: " + path.string());
  }
  if (!v.empty()) {
    f.write(reinterpret_cast<const char *>(v.data()),
            static_cast<std::streamsize>(v.size()));
  }
  f.flush();
  if (!f) {
    throw std::runtime_error("write failed: " + path.string());
  }
  // Verify written size (tellp after flush; fail-closed if short write).
  const auto pos = f.tellp();
  if (pos < 0 || static_cast<size_t>(pos) != v.size()) {
    throw std::runtime_error(
        "write size mismatch for " + path.string() + ": tellp=" +
        std::to_string(static_cast<long long>(pos)) + " expected=" +
        std::to_string(v.size()));
  }
  f.close();
  if (f.fail()) {
    throw std::runtime_error("close failed: " + path.string());
  }
}

static std::vector<std::uint8_t> pack_ifm_bf16_ones() {
  std::vector<std::uint8_t> out;
  out.reserve(kIfmBytes);
  const uint16_t one = f32_to_bf16_bits(1.0f);
  for (size_t i = 0; i < static_cast<size_t>(kM) * static_cast<size_t>(kK); ++i) {
    out.push_back(static_cast<std::uint8_t>(one & 0xff));
    out.push_back(static_cast<std::uint8_t>((one >> 8) & 0xff));
  }
  return out;
}

// Check size via filesystem metadata before allocating/reading the whole file.
static std::vector<std::uint8_t> read_file_bytes_exact(const fs::path &path,
                                                       size_t expect) {
  std::error_code ec;
  const auto sz = fs::file_size(path, ec);
  if (ec) {
    throw std::runtime_error("file_size failed: " + path.string() + ": " +
                             ec.message());
  }
  if (sz != expect) {
    throw std::runtime_error("file size " + std::to_string(sz) + " != expected " +
                             std::to_string(expect) + ": " + path.string());
  }
  std::ifstream f(path, std::ios::binary);
  if (!f) {
    throw std::runtime_error("read open failed: " + path.string());
  }
  std::vector<std::uint8_t> buf(expect);
  if (expect > 0) {
    f.read(reinterpret_cast<char *>(buf.data()),
           static_cast<std::streamsize>(expect));
    if (!f) {
      throw std::runtime_error("read failed: " + path.string());
    }
  }
  return buf;
}

static std::string escape_md_inline(const std::string &s) {
  std::string o;
  o.reserve(s.size());
  for (char c : s) {
    if (c == '|' || c == '`' || c == '\n' || c == '\r') {
      o.push_back(' ');
    } else {
      o.push_back(c);
    }
  }
  return o;
}

static void write_provenance(const fs::path &path, bool wts_from_file,
                             const std::string &wts_src) {
  std::ofstream f(path);
  if (!f) {
    throw std::runtime_error("open PROVENANCE failed: " + path.string());
  }
  f << "# PROVENANCE — mladf_ref lab buffers\n\n"
    << "**Lab only. NOT a silicon Pass. NOT product-ready weights.**\n\n"
    << "## License\n\n"
    << "AMD Ryzen AI / DynamicDispatch materials remain under AMD EULA when\n"
    << "copied from the vendor install. Zero-filled WTS is generated by this\n"
    << "tool (dragon-npu pack_mladf_ref) and is **not** an AMD-shuffled tensor.\n"
    << "Do not commit proprietary AMD-packed blobs to public git.\n\n"
    << "## Key\n\n"
    << "```\n"
    << kStagedKey << "\n"
    << "```\n\n"
    << "| Field | Value |\n"
    << "|-------|-------|\n"
    << "| M × K × N | " << kM << " × " << kK << " × " << kN << " |\n"
    << "| Group size (Gs) | " << kGs << " |\n"
    << "| IFM bytes | " << kIfmBytes << " (bf16 ones) |\n"
    << "| WTS bytes | " << kWtsBytes << " |\n"
    << "| OFM expected bytes | " << kOfmBytes << " (zeros placeholder) |\n"
    << "| Ctrl packet (not written) | " << kCtrlBytes << " |\n\n"
    << "## Files\n\n"
    << "| File | Content |\n"
    << "|------|---------|\n"
    << "| `ifm.bin` | bf16 LE ones, row-major M×K — **correct pack** |\n";
  if (wts_from_file) {
    f << "| `wts.bin` | Size-checked copy from operator path (not generated here) |\n"
      << "| `wts source` | `" << escape_md_inline(wts_src) << "` |\n";
  } else {
    f << "| `wts.bin` | **Zero-filled** length " << kWtsBytes
      << " — **NOT** AMD weight shuffle |\n";
  }
  f << "| `ofm_expected.bin` | Zeros length " << kOfmBytes
    << " — placeholder only; **not** a Pass golden |\n\n"
    << "## Honesty\n\n"
    << "- Sizes match AMD `*_ctrl_meta` for the staged key (IFM/WTS/OFM).\n"
    << "- f32 ones×ones golden is **invalid** for this a16w4 kernel.\n"
    << "- All-zero `ofm_expected.bin` is **refused** as Pass golden by default\n"
    << "  (`XDNA2_LAB_OFM_GOLDEN` + mladf decide) — would false-Pass vs zeroed OFM.\n"
    << "- Zero WTS is valid for BO plumbing tests only.\n"
    << "- Real WTS: `bash tools/export_mladf_wts.sh` (DD export_const_params),\n"
    << "  then `--wts-from <wts.bin>` or set `XDNA2_LAB_WTS_PACKED`.\n"
    << "- Ctrl packet (29664 B) is **not** produced here.\n"
    << "- Never claim `ran_on_npu` from packing alone.\n"
    << "- Constants must match Rust `STAGED_MLADF_*` "
       "(see `staged_sizes_match_pack_tool_contract` test).\n";
  f.flush();
  if (!f) {
    throw std::runtime_error("write PROVENANCE failed");
  }
}

static void usage(const char *argv0) {
  std::cerr
      << "Usage: " << argv0 << " [OUT_DIR] [--wts-from PATH]\n"
      << "  OUT_DIR default: testdata/xdna2_cmd/mladf_ref (via pack_mladf_ref.sh)\n"
      << "  --wts-from PATH  copy pre-packed WTS (must be exactly " << kWtsBytes
      << " bytes; size checked before full read)\n"
      << "  Env: XDNA2_PACK_WTS_FROM (same as --wts-from)\n"
      << "Exit non-zero if any written size is wrong.\n";
}

int main(int argc, char **argv) {
  std::string out_dir = "testdata/xdna2_cmd/mladf_ref";
  std::string wts_from;

  for (int i = 1; i < argc; ++i) {
    const std::string a = argv[i];
    if (a == "-h" || a == "--help") {
      usage(argv[0]);
      return 0;
    }
    if (a == "--wts-from") {
      if (i + 1 >= argc) {
        std::cerr << "ERROR: --wts-from needs a path\n";
        return 2;
      }
      wts_from = argv[++i];
      continue;
    }
    if (!a.empty() && a[0] == '-') {
      std::cerr << "ERROR: unknown flag " << a << "\n";
      usage(argv[0]);
      return 2;
    }
    out_dir = a;
  }

  if (wts_from.empty()) {
    if (const char *e = std::getenv("XDNA2_PACK_WTS_FROM")) {
      if (e[0] != '\0') {
        wts_from = e;
      }
    }
  }

  try {
    fs::create_directories(out_dir);

    auto ifm = pack_ifm_bf16_ones();
    if (ifm.size() != kIfmBytes) {
      std::cerr << "ERROR: IFM packed " << ifm.size() << " != " << kIfmBytes
                << "\n";
      return 1;
    }

    std::vector<std::uint8_t> wts;
    bool wts_from_file = false;
    if (!wts_from.empty()) {
      // Metadata length check before full read (Issue 8).
      wts = read_file_bytes_exact(wts_from, kWtsBytes);
      // Align with stage / validate_mladf_wts: refuse all-zero operator packs
      // (zero-fill remains the default only when --wts-from is unset).
      bool all_zero = true;
      for (auto b : wts) {
        if (b != 0) {
          all_zero = false;
          break;
        }
      }
      if (all_zero) {
        std::cerr
            << "ERROR: --wts-from is all zeros (not AMD-packed). "
               "Use bash tools/export_mladf_wts.sh or a real capture.\n";
        return 1;
      }
      wts_from_file = true;
    } else {
      wts.assign(kWtsBytes, 0);
    }

    std::vector<std::uint8_t> ofm(kOfmBytes, 0);

    const fs::path base(out_dir);
    write_bytes(base / "ifm.bin", ifm);
    write_bytes(base / "wts.bin", wts);
    write_bytes(base / "ofm_expected.bin", ofm);
    write_provenance(base / "PROVENANCE.md", wts_from_file, wts_from);

    auto check = [&](const char *name, size_t expect) {
      auto got = fs::file_size(base / name);
      if (got != expect) {
        std::cerr << "ERROR: " << name << " size " << got << " != " << expect
                  << "\n";
        std::exit(1);
      }
      std::cout << "  ok " << name << "  " << got << " B\n";
    };
    std::cout << "pack_mladf_ref → " << fs::absolute(base) << "\n";
    std::cout << "  key " << kStagedKey << "\n";
    std::cout << "  shape M=" << kM << " K=" << kK << " N=" << kN
              << " Gs=" << kGs << "\n";
    check("ifm.bin", kIfmBytes);
    check("wts.bin", kWtsBytes);
    check("ofm_expected.bin", kOfmBytes);
    if (wts_from_file) {
      std::cout << "  wts source: " << wts_from << " (size-checked copy)\n";
    } else {
      std::cout << "  wts: ZERO-FILLED (not AMD shuffle) — length OK for BO "
                   "plumbing only\n";
    }
    std::cout << "  wrote PROVENANCE.md\n";
    std::cout << "\nNext:\n"
              << "  export XDNA2_LAB_PACK=mladf_a16w4\n"
              << "  export XDNA2_LAB_WTS_PACKED=" << (base / "wts.bin") << "\n"
              << "  # ofm_expected zeros are NOT a Pass golden (refused by default)\n"
              << "  # export XDNA2_LAB_OFM_GOLDEN=/path/to/nonzero_ofm.bin\n";
    return 0;
  } catch (const std::exception &e) {
    std::cerr << "ERROR: " << e.what() << "\n";
    return 1;
  }
}
