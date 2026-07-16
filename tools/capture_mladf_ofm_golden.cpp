// capture_mladf_ofm_golden — Option A: vendor OFM dump for lab Pass golden.
//
// Runs AMD DynamicDispatch `mladfmatmulbias` on the **same** staged shape and
// **same** lab-synthetic host tensors as `export_mladf_wts`, with load_xrt=true,
// then writes OFM as raw bf16 LE bytes (512 for M=1 N=256).
//
// This is the instrumented capture path for Priority 1 — operator runs one
// script; no hand-hunting BO dumps.
//
// Requires: Ryzen AI DD + XRT + libaiebu + working NPU path (L3 may need
// temporary exec_cmd_allow=1 on hardened hosts). Does **not** set ALLOW itself.
// Does **not** claim dragon-npu ran_on_npu.
//
// Build + run: tools/capture_mladf_ofm_golden.sh

#include <ops/mladfmatmulbias/mladfmatmulbias.hpp>
#include <ops/ops_common/dtype_utils.h>

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

static constexpr const char *kStagedKey =
    "mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128";
static constexpr size_t kM = 1;
static constexpr size_t kK = 1152;
static constexpr size_t kN = 256;
static constexpr size_t kGs = 128;
static constexpr size_t kOfmBytes = 512; // M*N*2 bf16
static constexpr size_t kIfmElems = kM * kK;
static constexpr size_t kOfmElems = kM * kN;

static void write_bytes(const fs::path &path, const void *data, size_t n) {
  std::ofstream f(path, std::ios::binary);
  if (!f) {
    throw std::runtime_error("open failed: " + path.string());
  }
  if (n > 0) {
    f.write(reinterpret_cast<const char *>(data),
            static_cast<std::streamsize>(n));
  }
  f.flush();
  if (!f) {
    throw std::runtime_error("write failed: " + path.string());
  }
}

static bool bytes_all_zero(const std::uint8_t *p, size_t n) {
  for (size_t i = 0; i < n; ++i) {
    if (p[i] != 0) {
      return false;
    }
  }
  return n > 0;
}

static void write_provenance(const fs::path &path, size_t nz,
                             const std::string &note) {
  std::ofstream f(path);
  if (!f) {
    throw std::runtime_error("open PROVENANCE failed");
  }
  f << "# PROVENANCE — OFM golden capture (capture_mladf_ofm_golden)\n\n"
    << "**Lab Option A capture. NOT dragon-npu ran_on_npu by itself.**\n\n"
    << "## Source\n\n"
    << "AMD Ryzen AI DynamicDispatch `mladfmatmulbias::execute` with\n"
    << "`load_xrt=true`, same lab-synthetic host tensors as `export_mladf_wts`.\n\n"
    << "| Field | Value |\n"
    << "|-------|-------|\n"
    << "| Key target | `" << kStagedKey << "` |\n"
    << "| M × K × N × Gs | " << kM << " × " << kK << " × " << kN << " × " << kGs
    << " |\n"
    << "| OFM bytes | " << kOfmBytes << " (bf16 LE) |\n"
    << "| Non-zero bytes | " << nz << " |\n"
    << "| IFM | bf16 ones (matches dragon-npu lab fill) |\n"
    << "| WTS path | same synthetic int4/scales/zeros/bias → vendor pack |\n"
    << "| op_version | v2 |\n\n"
    << "## Honesty\n\n"
    << "- Golden is **vendor NPU OFM** for this lab tensor set.\n"
    << "- Pair with WTS from the same export (or re-export before capture).\n"
    << "- " << note << "\n"
    << "- Use: `bash tools/silicon_lab_operator.sh install-golden <ofm.bin>`\n"
    << "  or the capture script's auto-install into silicon_lab/.\n";
}

static void usage(const char *argv0) {
  std::cerr
      << "Usage: " << argv0 << " [OUT_DIR]\n"
      << "  Writes ofm_golden.bin (" << kOfmBytes
      << " B bf16) + PROVENANCE_OFM_CAPTURE.md\n"
      << "  Needs live XRT/NPU (load_xrt=true). Exit non-zero on fail.\n";
}

int main(int argc, char **argv) {
  std::string out_dir = "testdata/xdna2_cmd/mladf_ofm_capture";
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

    // Same scratch / synthetic host layout as export_mladf_wts.
    constexpr size_t kScratchBytes = 2 * 1024 * 1024;
    const size_t w_logical = (kK * kN) / 2;

    std::vector<std::int8_t> weights(kScratchBytes, 0);
    for (size_t i = 0; i < w_logical && i < weights.size(); ++i) {
      weights[i] = static_cast<std::int8_t>((i * 3 + 1) & 0xff);
    }
    std::vector<float> scales(kScratchBytes / sizeof(float), 1.0f);
    std::vector<std::int8_t> zeros(kScratchBytes, 0);
    std::vector<float> bias(kScratchBytes / sizeof(float), 0.0f);

    std::map<std::string, std::any> attr;
    attr["op_version"] = std::string("v2");
    attr["group_size"] = std::vector<int>{static_cast<int>(kGs)};

    // load_xrt=true — real vendor NPU path (Option A).
    ryzenai::mladfmatmulbias<std::uint16_t, std::int8_t, std::uint16_t,
                             std::uint16_t>
        op("bfloat16", "int4", "bfloat16", /*load_xrt=*/true, attr);

    Tensor tw{weights.data(), {kK, kN}, "int4"};
    Tensor ts{scales.data(), {kK / kGs, kN}, "float"};
    Tensor tz{zeros.data(), {kK / kGs, kN}, "int8"};
    Tensor tb{bias.data(), {1, kN}, "float"};

    op.initialize_const_params({tw, ts, tz, tb}, attr);
    op.set_shape({kM, kK}, {kK, kN}, static_cast<int>(kGs));

    // Lab IFM: bf16 ones (matches dragon-npu mladf pack ones fill).
    std::vector<std::uint16_t> ifm(kIfmElems);
    const std::uint16_t one_bf16 = ryzenai::float_to_bfloat16(1.0f);
    for (auto &v : ifm) {
      v = one_bf16;
    }
    std::vector<std::uint16_t> ofm(kOfmElems, 0);

    Tensor ti{ifm.data(), {kM, kK}, "bfloat16"};
    Tensor to{ofm.data(), {kM, kN}, "bfloat16"};
    std::vector<Tensor> inputs{ti};
    std::vector<Tensor> outputs{to};

    std::cout << "capture_mladf_ofm_golden: execute load_xrt=true "
              << "M=" << kM << " K=" << kK << " N=" << kN << " …\n";
    op.execute(inputs, outputs);

    const auto *raw = reinterpret_cast<const std::uint8_t *>(ofm.data());
    if (kOfmElems * sizeof(std::uint16_t) != kOfmBytes) {
      throw std::runtime_error("internal OFM size mismatch");
    }
    if (bytes_all_zero(raw, kOfmBytes)) {
      std::cerr << "ERROR: OFM is all zeros after execute — refuse as Pass golden\n";
      std::cerr << "  Hint: NPU may not have run (L3 exec_cmd_allow? XRT device?)\n";
      return 1;
    }

    size_t nz = 0;
    for (size_t i = 0; i < kOfmBytes; ++i) {
      if (raw[i] != 0) {
        ++nz;
      }
    }

    const fs::path base(out_dir);
    write_bytes(base / "ofm_golden.bin", raw, kOfmBytes);
    // Also dump IFM for provenance/debug (not required by lab).
    write_bytes(base / "ifm_ones_bf16.bin", ifm.data(),
                kIfmElems * sizeof(std::uint16_t));
    write_provenance(base / "PROVENANCE_OFM_CAPTURE.md", nz,
                     "Pair with WTS from export_mladf_wts using the same synthetic weights.");

    std::cout << "capture_mladf_ofm_golden → " << fs::absolute(base) << "\n";
    std::cout << "  key target: " << kStagedKey << "\n";
    std::cout << "  ok ofm_golden.bin  " << kOfmBytes << " B  nonzero_bytes=" << nz
              << "\n";
    std::cout << "  wrote PROVENANCE_OFM_CAPTURE.md\n";
    std::cout << "\nNext:\n"
              << "  bash tools/validate_mladf_ofm_golden.sh "
              << (base / "ofm_golden.bin") << "\n"
              << "  bash tools/silicon_lab_operator.sh install-golden "
              << (base / "ofm_golden.bin") << "\n";
    return 0;
  } catch (const std::exception &e) {
    std::cerr << "ERROR: " << e.what() << "\n";
    std::cerr
        << "  Hint: Option A needs live XRT/NPU. On hardened hosts:\n"
        << "    cat /sys/module/amdxdna/parameters/exec_cmd_allow  # may need 1\n"
        << "    temporary: echo 1 | sudo tee …/exec_cmd_allow\n"
        << "    re-run this tool; then echo 0 | sudo tee …/exec_cmd_allow\n"
        << "  DD+XRT+aiebu link: same as export_mladf_wts.sh\n";
    return 1;
  }
}
