// export_mladf_wts — offline AMD DynamicDispatch weight pack (lab only).
//
// Calls vendor `mladfmatmulbias::export_const_params` (closed lib) to produce a
// real BFP_UNIFIED-shuffled WTS blob for the staged lab key:
//   M=1 K=1152 N=256 Gs=128 → STAGED_MLADF_WTS_BYTES = 174080
//
// Requires Ryzen AI DynamicDispatch install + system XRT + libaiebu.
// Does **not** invent a pure-Rust fake shuffle.
// Does **not** claim silicon Pass / ran_on_npu.
//
// Build + run: tools/export_mladf_wts.sh
//
// Tensor layout (probed on Ryzen AI 1.7.x DD):
//   const_params[0] weights  int4 packed as int8, shape [K, N]  (K*N/2 bytes)
//   const_params[1] scales   float32, shape [K/Gs, N]
//   const_params[2] zeros    int8,    shape [K/Gs, N]
//   const_params[3] bias     float32, shape [1, N]
//
// Note: constructor attr op_version="flat" is refused by this DD build
// ("flat version is not supported for matmulbias"). We use op_version="v2",
// which selects the same MatmulWtsShuffle::BFP_UNIFIED path as flat/v2 in
// open headers (utils.hpp). Output size must still match 174080.

#include <ops/mladfmatmulbias/mladfmatmulbias.hpp>

#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <map>
#include <any>
#include <stdexcept>
#include <string>
#include <vector>

namespace fs = std::filesystem;

// Must match src/backends/amd_xdna2/mladf_pack.rs STAGED_MLADF_*
static constexpr const char *kStagedKey =
    "mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128";
static constexpr size_t kM = 1;
static constexpr size_t kK = 1152;
static constexpr size_t kN = 256;
static constexpr size_t kGs = 128;
static constexpr size_t kWtsBytes = 174080; // STAGED_MLADF_WTS_BYTES

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
  f.close();
  if (f.fail()) {
    throw std::runtime_error("close failed: " + path.string());
  }
}

static bool bytes_all_zero(const std::vector<std::uint8_t> &v) {
  if (v.empty()) {
    return true;
  }
  for (auto b : v) {
    if (b != 0) {
      return false;
    }
  }
  return true;
}

static void write_provenance(const fs::path &path, size_t nz_bytes,
                             const std::string &note) {
  std::ofstream f(path);
  if (!f) {
    throw std::runtime_error("open PROVENANCE failed: " + path.string());
  }
  f << "# PROVENANCE — AMD-packed mladf WTS (export_mladf_wts)\n\n"
    << "**Lab only. NOT a silicon Pass. NOT product weights for deployment.**\n\n"
    << "## License\n\n"
    << "Packed via AMD Ryzen AI DynamicDispatch `export_const_params` under the\n"
    << "operator's AMD EULA install. Do **not** commit this binary to public git.\n\n"
    << "## Key (target)\n\n"
    << "```\n"
    << kStagedKey << "\n"
    << "```\n\n"
    << "| Field | Value |\n"
    << "|-------|-------|\n"
    << "| M × K × N × Gs | " << kM << " × " << kK << " × " << kN << " × " << kGs
    << " |\n"
    << "| WTS bytes | " << kWtsBytes << " |\n"
    << "| Non-zero bytes | " << nz_bytes << " |\n"
    << "| op_version attr | v2 (flat refused by this DD; BFP_UNIFIED shuffle) |\n"
    << "| Source | vendor `mladfmatmulbias::export_const_params` |\n"
    << "| Synthetic host tensors | int4 ones-ish + float scales=1 + int8 zeros + float bias=0 |\n\n"
    << "## Honesty\n\n"
    << "- This is a **real AMD shuffle**, not pure-Rust fake packing.\n"
    << "- Host weights are **lab synthetic** (not a trained model).\n"
    << "- " << note << "\n"
    << "- Never claim `ran_on_npu` from packing alone.\n"
    << "- Wire with: `export XDNA2_LAB_WTS_PACKED=<this>/wts.bin` and\n"
    << "  `XDNA2_LAB_REQUIRE_NONZERO_WTS=1`.\n";
  f.flush();
  if (!f) {
    throw std::runtime_error("write PROVENANCE failed");
  }
}

static void usage(const char *argv0) {
  std::cerr
      << "Usage: " << argv0 << " [OUT_DIR]\n"
      << "  OUT_DIR default: testdata/xdna2_cmd/mladf_wts_export (via .sh)\n"
      << "Writes wts.bin (" << kWtsBytes
      << " B, non-zero) + PROVENANCE.md\n"
      << "Exit non-zero if size wrong, all-zero, or DD/XRT missing at link/run.\n";
}

int main(int argc, char **argv) {
  std::string out_dir = "testdata/xdna2_cmd/mladf_wts_export";
  for (int i = 1; i < argc; ++i) {
    const std::string a = argv[i];
    if (a == "-h" || a == "--help") {
      usage(argv[0]);
      return 0;
    }
    if (!a.empty() && a[0] == '-') {
      std::cerr << "ERROR: unknown flag " << a << "\n";
      usage(argv[0]);
      return 2;
    }
    out_dir = a;
  }

  try {
    fs::create_directories(out_dir);

    // Host tensors for staged shape.
    //
    // IMPORTANT: on Ryzen AI 1.7.x DD, export_const_params / const_params_shuffle
    // over-reads beyond the logical [K,N] host buffers (pad/tile math). Exact
    // logical sizes (K*N/2 for int4, (K/Gs)*N for scales) segfault. Allocate a
    // generous scratch arena; only the logical shape fields are authoritative.
    constexpr size_t kScratchBytes = 2 * 1024 * 1024; // 2 MiB
    const size_t w_logical = (kK * kN) / 2;           // int4 packed logical
    const size_t scale_logical = (kK / kGs) * kN;

    std::vector<std::int8_t> weights(kScratchBytes, 0);
    for (size_t i = 0; i < w_logical && i < weights.size(); ++i) {
      // Non-zero deterministic pattern (lab only).
      weights[i] = static_cast<std::int8_t>((i * 3 + 1) & 0xff);
    }
    std::vector<float> scales(kScratchBytes / sizeof(float), 1.0f);
    std::vector<std::int8_t> zeros(kScratchBytes, 0);
    std::vector<float> bias(kScratchBytes / sizeof(float), 0.0f);
    (void)scale_logical;

    std::map<std::string, std::any> attr;
    // flat is refused on current DD; v2 → BFP_UNIFIED (same as flat in headers).
    attr["op_version"] = std::string("v2");
    attr["group_size"] = std::vector<int>{static_cast<int>(kGs)};

    ryzenai::mladfmatmulbias<std::uint16_t, std::int8_t, std::uint16_t,
                             std::uint16_t>
        op("bfloat16", "int4", "bfloat16", /*load_xrt=*/false, attr);

    Tensor tw{weights.data(), {kK, kN}, "int4"};
    Tensor ts{scales.data(), {kK / kGs, kN}, "float"};
    Tensor tz{zeros.data(), {kK / kGs, kN}, "int8"};
    Tensor tb{bias.data(), {1, kN}, "float"};

    const auto packed = op.export_const_params({tw, ts, tz, tb}, attr);
    if (packed.empty()) {
      std::cerr << "ERROR: export_const_params returned no blobs\n";
      return 1;
    }

    const auto &wts = packed[0];
    if (wts.size() != kWtsBytes) {
      std::cerr << "ERROR: WTS blob size " << wts.size() << " != " << kWtsBytes
                << " (staged contract)\n";
      return 1;
    }
    if (bytes_all_zero(wts)) {
      std::cerr << "ERROR: WTS blob is all zeros — refuse (not a usable pack)\n";
      return 1;
    }

    size_t nz = 0;
    for (auto b : wts) {
      if (b != 0) {
        ++nz;
      }
    }

    const fs::path base(out_dir);
    write_bytes(base / "wts.bin", wts);
    // Optional sibling blobs (not used by lab WTS path).
    for (size_t i = 1; i < packed.size(); ++i) {
      write_bytes(base / ("extra_blob_" + std::to_string(i) + ".bin"), packed[i]);
    }
    write_provenance(base / "PROVENANCE.md", nz,
                     "extra_blob_*.bin may be kernel side-data; lab uses wts.bin only.");

    // Re-check on disk.
    const auto got = fs::file_size(base / "wts.bin");
    if (got != kWtsBytes) {
      std::cerr << "ERROR: written wts.bin size " << got << " != " << kWtsBytes
                << "\n";
      return 1;
    }

    std::cout << "export_mladf_wts → " << fs::absolute(base) << "\n";
    std::cout << "  key target: " << kStagedKey << "\n";
    std::cout << "  shape M=" << kM << " K=" << kK << " N=" << kN << " Gs=" << kGs
              << "\n";
    std::cout << "  ok wts.bin  " << kWtsBytes << " B  nonzero_bytes=" << nz
              << "\n";
    std::cout << "  wrote PROVENANCE.md\n";
    std::cout << "\nNext (still not Pass):\n"
              << "  bash tools/validate_mladf_wts.sh " << (base / "wts.bin")
              << "\n"
              << "  bash tools/pack_mladf_ref.sh --wts-from " << (base / "wts.bin")
              << "\n"
              << "  # or stage:\n"
              << "  bash tools/stage_silicon_lab_assets.sh --wts-from "
              << (base / "wts.bin") << "\n"
              << "  export XDNA2_LAB_WTS_PACKED=" << (base / "wts.bin") << "\n"
              << "  export XDNA2_LAB_REQUIRE_NONZERO_WTS=1\n";
    return 0;
  } catch (const std::exception &e) {
    std::cerr << "ERROR: " << e.what() << "\n";
    std::cerr << "  Hint: need Ryzen AI DD + XRT + libaiebu; see "
                 "docs/SILICON_PATH_LAB.md Phase A.\n";
    return 1;
  }
}
