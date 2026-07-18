// capture_mladf_instr_vendor — dump vendor instruction BO after working NPU matmul.
//
// Same lab tensors / key as capture_mladf_ofm_golden (load_xrt=true). After a
// non-zero OFM execute:
//   1) get_transaction_bin with inputs={IFM,WTS} — classic 1688 B →
//      instr_vendor_txn_bin.bin; oversized multi-txn (e.g. 32k) →
//      instr_vendor_get_transaction_bin.bin only (diagnostic, wrong body)
//   2) Always dump Transaction::get_txn_bvec(staged key + siblings) → prefer
//      classic 1688 as instr_vendor_txn_bin.bin (overwrites multi-txn primary)
//   3) Optional get_ctrl_pkts via OpInterface*
//   4) Try instr_in_registry / get_instr_bo for staged + GM sibling keys;
//      always attempt get_instr_bo even when instr_in_registry reports miss
//   5) Prefer registry BO as instr_vendor_post_patch.bin; else classic 1688
//      unpatched (never write multi-txn as post_patch when classic map exists)
//
// Research (IMPL 0eea644f — offline, DD 1.7.1 libdyn_dispatch_core.so):
//   INSTR_BO_LIMIT enforced in instruction_cache::put when
//     align_to_next(txn_len+8, 0x8000) > capacity_
//   capacity_ ctor default = 0x3c00000 (60 MiB); align quantum = 0x8000 (32 KiB).
//   classic 1688 + multi 32208 both fit default capacity — HITL miss ≠ size bug.
//   get_instr_bo → cache.get → cache.put(Transaction::get_txn_str) — does NOT
//   map the live execute instr BO. Mode registry_get_instr_bo = Transaction/cache
//   host image only — NOT proven live-patched run_ BO after execute.
//   mladfmatmulbias: no public instr BO after execute (IFM/WTS/OFM/const/ctrl;
//   run_ private). Multi ~32208 = classic-format wrong body (500 ops);
//   find(classic1688)==-1; not ELF. Residual = XRT/DD architecture + APP_LOAD.
//
// Writes under OUT_DIR (default testdata/xdna2_cmd/gt_capture):
//   instr_vendor_post_patch.bin  — registry post-patch OR classic 1688 unpatched
//   instr_vendor_txn_bin.bin     — best classic unpatched 1688 when available
//   instr_vendor_get_transaction_bin.bin — oversized multi-txn diagnostic only
//   ctrl_vendor_pkts.bin         — optional ctrl packets
//   vendor_paddrs.json           — schema v1 + capture_mode + paddrs (may be null)
//   ofm_golden.bin               — non-zero OFM proves NPU ran
//   PROVENANCE_VENDOR_INSTR.md
//
// Honesty: NOT dragon-npu ran_on_npu. Needs temporary L3 on hardened hosts.
// Does not set XDNA2_ALLOW_EXEC_CMD. Does not invent vendor bytes offline.
// CAPTURE_MODE_UNPATCHED / CLASS A ≠ C-ready.

#include "txn/txn_container.hpp"

#include <ops/mladfmatmulbias/mladfmatmulbias.hpp>
#include <ops/ops_common/dtype_utils.h>
#include <ops/op_interface.hpp>

#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <map>
#include <any>
#include <sstream>
#include <stdexcept>
#include <string>
#include <vector>

#include <fcntl.h>
#include <sys/wait.h>
#include <unistd.h>

namespace fs = std::filesystem;

static constexpr const char *kStagedKey =
    "mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128";
static constexpr size_t kM = 1;
static constexpr size_t kK = 1152;
static constexpr size_t kN = 256;
static constexpr size_t kGs = 128;
static constexpr size_t kOfmBytes = 512;
static constexpr size_t kIfmElems = kM * kK;
static constexpr size_t kOfmElems = kM * kN;
// Lab pure-direct content size for this staged key (content mode).
static constexpr size_t kMinInstrBytes = 1688;
// HITL get_transaction_bin wrong-body size (classic header layout, 500 ops).
static constexpr size_t kHitlMultiTxnBytes = 32208;
// DD instruction_cache (libdyn_dispatch_core.so) — research constants.
// INSTR_BO_LIMIT trip: align_to_next(getInstrBufSize(txn)=len+8, kInstrBoAlign)
//   > capacity_  (capacity ctor default kInstrBoCapacity).
static constexpr size_t kInstrBoAlign = 0x8000;       // 32 KiB
static constexpr size_t kInstrBoCapacity = 0x3c00000; // 60 MiB

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

// Empty (n==0) is treated as zero/invalid for dump accept (Issue 8).
static bool bytes_all_zero(const std::uint8_t *p, size_t n) {
  if (n == 0) {
    return true;
  }
  for (size_t i = 0; i < n; ++i) {
    if (p[i] != 0) {
      return false;
    }
  }
  return true;
}

// Sibling map keys must never become instr_vendor_post_patch / primary txn.
static bool is_sibling_suffix_key(const std::string &key) {
  auto ends_with = [&](const char *suf) {
    const size_t n = std::char_traits<char>::length(suf);
    return key.size() >= n && key.compare(key.size() - n, n, suf) == 0;
  };
  return ends_with("_ctrl_meta") || ends_with("_ctrl") || ends_with("_param");
}

// Primary matmul keys only (not ctrl/meta/param siblings).
static bool is_primary_instr_key(const std::string &key) {
  return !key.empty() && !is_sibling_suffix_key(key);
}

static size_t nonzero_count(const std::uint8_t *p, size_t n) {
  size_t nz = 0;
  for (size_t i = 0; i < n; ++i) {
    if (p[i] != 0) {
      ++nz;
    }
  }
  return nz;
}

// Minimal sha256 via sha256sum(1) for provenance (no new deps).
// fork/exec + pipe — never shell-interpolates path (popen with unquoted path was unsafe).
static std::string sha256_file(const fs::path &path) {
  int pipefd[2];
  if (pipe(pipefd) != 0) {
    return "ERR";
  }
  pid_t pid = fork();
  if (pid < 0) {
    close(pipefd[0]);
    close(pipefd[1]);
    return "ERR";
  }
  if (pid == 0) {
    close(pipefd[0]);
    if (dup2(pipefd[1], STDOUT_FILENO) < 0) {
      _exit(127);
    }
    close(pipefd[1]);
    // Redirect stderr away from parent noise
    int devnull = open("/dev/null", O_WRONLY);
    if (devnull >= 0) {
      dup2(devnull, STDERR_FILENO);
      close(devnull);
    }
    execlp("sha256sum", "sha256sum", "--", path.c_str(),
           static_cast<char *>(nullptr));
    _exit(127);
  }
  close(pipefd[1]);
  char buf[256] = {};
  ssize_t n = read(pipefd[0], buf, sizeof(buf) - 1);
  close(pipefd[0]);
  int st = 0;
  waitpid(pid, &st, 0);
  if (n <= 0 || !WIFEXITED(st) || WEXITSTATUS(st) != 0) {
    return "ERR";
  }
  std::string s(buf, static_cast<size_t>(n));
  auto sp = s.find(' ');
  if (sp != std::string::npos) {
    s.resize(sp);
  } else {
    auto tab = s.find('\t');
    if (tab != std::string::npos) {
      s.resize(tab);
    }
  }
  while (!s.empty() && (s.back() == '\n' || s.back() == '\r')) {
    s.pop_back();
  }
  return s.empty() ? "ERR" : s;
}

static std::string json_escape(const std::string &s) {
  std::string o;
  o.reserve(s.size() + 8);
  for (char c : s) {
    if (c == '"' || c == '\\') {
      o.push_back('\\');
      o.push_back(c);
    } else if (c == '\n') {
      o += "\\n";
    } else {
      o.push_back(c);
    }
  }
  return o;
}

static void usage(const char *argv0) {
  std::cerr
      << "Usage: " << argv0 << " [OUT_DIR]\n"
      << "  After vendor NPU execute, dump instruction BO + OFM golden.\n"
      << "  Default OUT_DIR: testdata/xdna2_cmd/gt_capture\n"
      << "  Exit: 0 ok; 1 fail (OFM zero / dump fail / exception); 2 usage\n";
}

// Best-effort XRT BO device address (0 → unknown).
static std::uint64_t try_bo_addr(xrt::bo &bo) {
  try {
    const auto a = bo.address();
    if (a == 0 || a == static_cast<std::uint64_t>(-1)) {
      return 0;
    }
    return static_cast<std::uint64_t>(a);
  } catch (...) {
    return 0;
  }
}

static std::string hex_or_null(std::uint64_t a) {
  if (a == 0) {
    return "null";
  }
  std::ostringstream os;
  os << "\"0x" << std::hex << a << "\"";
  return os.str();
}

static void print_l3_restore_reminder() {
  std::cerr << "  Restore L3 if opened:\n"
            << "    sudo sh -c 'echo 0 > "
               "/sys/module/amdxdna/parameters/exec_cmd_allow'\n";
}

// Prefer primary matmul txn (≥kMinInstrBytes, non-JSON-ish) over ctrl siblings.
static bool prefer_as_instr_payload(const std::vector<std::uint8_t> &bytes) {
  if (bytes.empty() || bytes.size() < kMinInstrBytes) {
    return false;
  }
  if (bytes_all_zero(bytes.data(), bytes.size())) {
    return false;
  }
  // JSON meta starts with '{' after optional whitespace — refuse as instr.
  size_t i = 0;
  while (i < bytes.size() &&
         (bytes[i] == ' ' || bytes[i] == '\t' || bytes[i] == '\n' ||
          bytes[i] == '\r')) {
    ++i;
  }
  if (i < bytes.size() && (bytes[i] == '{' || bytes[i] == '[')) {
    return false;
  }
  return true;
}

// Classic staged-key content size (ctrltext = 1688). Prefer over multi-txn.
static bool is_classic_instr_size(size_t n) { return n == kMinInstrBytes; }

// Page-padded BO (4096) is still a single classic payload class; multi-txn is
// larger (HITL saw 32208) and has a different op body — do not treat as primary.
static bool is_oversized_multi_txn(size_t n) {
  return n > 4096;
}

// Prefer classic 1688 (or page-pad 4096) for instr_vendor_txn_bin / unpatched
// post_patch. Multi-txn get_transaction_bin is diagnostic-only when classic
// exists.
static bool prefer_as_classic_instr_payload(
    const std::vector<std::uint8_t> &bytes) {
  if (!prefer_as_instr_payload(bytes)) {
    return false;
  }
  return is_classic_instr_size(bytes.size()) || bytes.size() == 4096;
}

// Minimal XAie_TxnHeader fields (LE) for diagnostic logs only.
struct TxnHdrDiag {
  std::uint8_t major = 0;
  std::uint8_t minor = 0;
  std::uint8_t dev_gen = 0;
  std::uint32_t num_ops = 0;
  std::uint32_t txn_size = 0;
  bool ok = false;
};

static TxnHdrDiag parse_txn_hdr_diag(const std::uint8_t *p, size_t n) {
  TxnHdrDiag d;
  if (p == nullptr || n < 16) {
    return d;
  }
  d.major = p[0];
  d.minor = p[1];
  d.dev_gen = p[2];
  d.num_ops = static_cast<std::uint32_t>(p[8]) |
              (static_cast<std::uint32_t>(p[9]) << 8) |
              (static_cast<std::uint32_t>(p[10]) << 16) |
              (static_cast<std::uint32_t>(p[11]) << 24);
  d.txn_size = static_cast<std::uint32_t>(p[12]) |
               (static_cast<std::uint32_t>(p[13]) << 8) |
               (static_cast<std::uint32_t>(p[14]) << 16) |
               (static_cast<std::uint32_t>(p[15]) << 24);
  d.ok = true;
  return d;
}

static void log_txn_hdr_diag(const char *label, const std::uint8_t *p,
                             size_t n) {
  const auto d = parse_txn_hdr_diag(p, n);
  if (!d.ok) {
    std::cout << "  " << label << " hdr: too short size=" << n << "\n";
    return;
  }
  const bool elf = n >= 4 && p[0] == 0x7f && p[1] == 'E' && p[2] == 'L' &&
                   p[3] == 'F';
  std::cout << "  " << label << " hdr: size=" << n
            << " maj/min=" << static_cast<int>(d.major) << "/"
            << static_cast<int>(d.minor) << " dev_gen="
            << static_cast<int>(d.dev_gen) << " num_ops=" << d.num_ops
            << " txn_size=" << d.txn_size << " elf=" << (elf ? "1" : "0")
            << (is_classic_instr_size(n) && d.num_ops == 40
                    ? " profile=staged_classic_1688"
                    : "")
            << (n == kHitlMultiTxnBytes && d.num_ops == 500
                    ? " profile=hitl_multi_wrong_body"
                    : "")
            << "\n";
}

// Research: multi does not embed classic 1688 (cannot peel slice).
static bool bytes_contain_slice(const std::uint8_t *hay, size_t hay_n,
                                const std::uint8_t *needle, size_t needle_n) {
  if (needle == nullptr || needle_n == 0 || hay == nullptr ||
      hay_n < needle_n) {
    return false;
  }
  for (size_t i = 0; i + needle_n <= hay_n; ++i) {
    if (std::memcmp(hay + i, needle, needle_n) == 0) {
      return true;
    }
  }
  return false;
}

// Always-on research note: default capacity fits classic+multi; miss ≠ size bug.
// err_msg empty → context-only; non-empty → annotate the thrown string.
static void log_instr_bo_limit_research(const std::string &err_msg,
                                        size_t attempted_hint_bytes) {
  const bool mentions_limit =
      !err_msg.empty() &&
      (err_msg.find("INSTR_BO_LIMIT") != std::string::npos ||
       err_msg.find("Instruction buffer size") != std::string::npos);
  std::cout << "  INSTR_BO_LIMIT research note:\n"
            << "    DD instruction_cache::put refuses when "
               "align_to_next(txn_len+8, "
            << kInstrBoAlign << ") > capacity_ (default capacity="
            << kInstrBoCapacity << " = 60MiB)\n"
            << "    align quantum=" << kInstrBoAlign
            << " (32KiB); classic 1688 AND multi 32208 BOTH FIT default 60MiB\n"
            << "    → HITL registry miss/err is NOT pure-direct size bug / not "
               "“1688 > INSTR_BO_LIMIT” under default capacity\n"
            << "    get_instr_bo = Transaction/cache load only — NOT live "
               "execute run_ instr BO (not proven live-patched)\n"
            << "    mladfmatmulbias: no public live instr BO after execute "
               "(IFM/WTS/OFM/const/ctrl only; run_ private)\n"
            << "    err_mentions_INSTR_BO_LIMIT="
            << (mentions_limit ? "1" : "0") << " attempted_hint_bytes="
            << attempted_hint_bytes << "\n";
  if (!err_msg.empty()) {
    std::cout << "    err: " << err_msg << "\n";
  }
}

// Dump unpatched AMD txn from Transaction singleton map (libtransaction.a).
// Primary keys only for instr_vendor_txn_bin.bin; siblings dump under
// txn_map_*.bin only (never as post_patch source).
// Prefer classic 1688 (staged key) over oversized multi-txn blobs.
// Returns true if a usable classic primary payload was written.
static bool dump_txn_container_keys(const fs::path &base,
                                    const std::vector<std::string> &keys,
                                    size_t *out_sz, size_t *out_nz,
                                    std::string *out_key,
                                    std::ostringstream *attempts,
                                    std::vector<std::uint8_t> *out_bytes) {
  auto &txn = Transaction::getInstance();
  bool any = false;
  for (const auto &key : keys) {
    // Never accept *_ctrl / *_ctrl_meta / *_param as primary txn (Issue 2).
    if (is_sibling_suffix_key(key)) {
      try {
        auto bytes = txn.get_txn_bvec(key);
        if (!bytes.empty() && key.size() > 5 &&
            key.compare(key.size() - 5, 5, "_ctrl") == 0 &&
            !bytes_all_zero(bytes.data(), bytes.size())) {
          write_bytes(base / ("txn_map_" + key + ".bin"), bytes.data(),
                      bytes.size());
          std::cout << "    txn_map sibling ctrl dump: " << key
                    << " size=" << bytes.size() << " (not primary)\n";
          if (attempts) {
            *attempts << "txn_map_sibling:" << key << ";";
          }
        }
      } catch (...) {
        // ignore sibling miss
      }
      continue;
    }
    try {
      auto bytes = txn.get_txn_bvec(key);
      if (bytes.empty() || bytes_all_zero(bytes.data(), bytes.size())) {
        std::cout << "    txn_map empty/zero: " << key << "\n";
        if (attempts) {
          *attempts << "txn_map_empty:" << key << ";";
        }
        continue;
      }
      const size_t nz = nonzero_count(bytes.data(), bytes.size());
      std::cout << "    txn_map hit: " << key << " size=" << bytes.size()
                << " nonzero=" << nz;
      // Prefer classic 1688 / page-pad 4096; refuse multi-txn as primary.
      if (!prefer_as_classic_instr_payload(bytes)) {
        if (prefer_as_instr_payload(bytes) && is_oversized_multi_txn(bytes.size())) {
          write_bytes(base / ("txn_map_multi_" + key + ".bin"), bytes.data(),
                      bytes.size());
          std::cout << " (multi-txn diagnostic only — not primary classic)\n";
          if (attempts) {
            *attempts << "txn_map_multi:" << key << ";";
          }
        } else {
          std::cout << " (skip as primary — need classic " << kMinInstrBytes
                    << "B non-JSON non-zero)\n";
          if (attempts) {
            *attempts << "txn_map_skip:" << key << ";";
          }
        }
        continue;
      }
      std::cout << "\n";
      // Primary dump path for classic unpatched instr source
      write_bytes(base / "instr_vendor_txn_bin.bin", bytes.data(),
                  bytes.size());
      if (out_sz) {
        *out_sz = bytes.size();
      }
      if (out_nz) {
        *out_nz = nz;
      }
      if (out_key) {
        *out_key = key;
      }
      if (out_bytes) {
        *out_bytes = bytes;
      }
      if (attempts) {
        *attempts << "txn_map_DUMPED:" << key << ";";
      }
      any = true;
      // Prefer first usable classic primary (keys ordered: staged first)
      break;
    } catch (const std::exception &e) {
      std::cout << "    txn_map miss/err: " << key << " : " << e.what()
                << "\n";
      if (attempts) {
        *attempts << "txn_map_err:" << key << ";";
      }
    }
  }
  return any;
}

static void dedup_keys(std::vector<std::string> &keys) {
  std::vector<std::string> uniq;
  for (const auto &k : keys) {
    if (k.empty()) {
      continue;
    }
    bool seen = false;
    for (const auto &u : uniq) {
      if (u == k) {
        seen = true;
        break;
      }
    }
    if (!seen) {
      uniq.push_back(k);
    }
  }
  keys.swap(uniq);
}

int main(int argc, char **argv) {
  std::string out_dir = "testdata/xdna2_cmd/gt_capture";
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
    const fs::path base(out_dir);

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

    ryzenai::mladfmatmulbias<std::uint16_t, std::int8_t, std::uint16_t,
                             std::uint16_t>
        op("bfloat16", "int4", "bfloat16", /*load_xrt=*/true, attr);

    Tensor tw{weights.data(), {kK, kN}, "int4"};
    Tensor ts{scales.data(), {kK / kGs, kN}, "float"};
    Tensor tz{zeros.data(), {kK / kGs, kN}, "int8"};
    Tensor tb{bias.data(), {1, kN}, "float"};

    op.initialize_const_params({tw, ts, tz, tb}, attr);
    op.set_shape({kM, kK}, {kK, kN}, static_cast<int>(kGs));

    std::vector<std::uint16_t> ifm(kIfmElems);
    const std::uint16_t one_bf16 = ryzenai::float_to_bfloat16(1.0f);
    for (auto &v : ifm) {
      v = one_bf16;
    }
    std::vector<std::uint16_t> ofm(kOfmElems, 0);

    Tensor ti{ifm.data(), {kM, kK}, "bfloat16"};
    Tensor to{ofm.data(), {kM, kN}, "bfloat16"};
    // execute uses IFM-only (vendor path); get_transaction_bin / fit_MKN needs
    // IFM + weight tensor (≥2 inputs) — HITL 3378dd7b root cause.
    std::vector<Tensor> inputs_exec{ti};
    std::vector<Tensor> inputs_txn{ti, tw};
    std::vector<Tensor> outputs{to};

    std::cout << "capture_mladf_instr_vendor: execute load_xrt=true "
              << "M=" << kM << " K=" << kK << " N=" << kN << " …\n";
    op.execute(inputs_exec, outputs);

    const auto *ofm_raw = reinterpret_cast<const std::uint8_t *>(ofm.data());
    if (bytes_all_zero(ofm_raw, kOfmBytes)) {
      std::cerr << "ERROR: OFM all zeros — NPU likely did not run (L3?)\n";
      std::cerr << "  Open L3: sudo sh -c 'echo 1 > "
                   "/sys/module/amdxdna/parameters/exec_cmd_allow'\n";
      print_l3_restore_reminder();
      return 1;
    }
    write_bytes(base / "ofm_golden.bin", ofm_raw, kOfmBytes);
    write_bytes(base / "ifm_ones_bf16.bin", ifm.data(),
                kIfmElems * sizeof(std::uint16_t));
    std::cout << "  OFM non-zero OK (" << nonzero_count(ofm_raw, kOfmBytes)
              << " nonzero bytes)\n";

    // Best-effort live BO paddrs (vendor XRT; may be host UVA or device).
    std::uint64_t ifm_paddr = 0;
    std::uint64_t wts_paddr = 0;
    std::uint64_t ofm_paddr = 0;
    std::uint64_t ctrl_paddr = 0;
    try {
      xrt::bo a = op.get_input(static_cast<int>(kM));
      xrt::bo c = op.get_output(static_cast<int>(kM));
      ifm_paddr = try_bo_addr(a);
      ofm_paddr = try_bo_addr(c);
      auto wts = op.get_const();
      if (!wts.empty()) {
        wts_paddr = try_bo_addr(wts[0]);
      }
      std::cout << "  bo_addrs ifm=" << hex_or_null(ifm_paddr)
                << " wts=" << hex_or_null(wts_paddr)
                << " ofm=" << hex_or_null(ofm_paddr) << "\n";
    } catch (const std::exception &e) {
      std::cout << "  bo_addrs unavailable: " << e.what() << "\n";
    }

    // Cached unpatched payload for post_patch fallbacks (avoid re-call errors).
    // Prefer classic 1688; multi-txn get_transaction_bin is diagnostic-only.
    std::vector<std::uint8_t> unpatched_bytes;
    std::string unpatched_source; // get_transaction_bin | txn_container
    size_t txn_sz = 0;
    size_t txn_nz = 0;
    bool txn_ok = false;         // classic unpatched primary ready
    bool classic_ok = false;     // unpatched is classic 1688 (or page-pad 4096)
    size_t multi_txn_sz = 0;     // oversized get_transaction_bin diagnostic

    // --- 1) get_transaction_bin with IFM+WTS (fit_MKN needs ≥2 inputs) ---
    // Classic 1688 → primary txn_bin. Oversized multi-txn (e.g. 32k) →
    // instr_vendor_get_transaction_bin.bin only (wrong body for staged key).
    try {
      auto txn = op.get_transaction_bin(inputs_txn, outputs, attr);
      if (prefer_as_classic_instr_payload(txn)) {
        write_bytes(base / "instr_vendor_txn_bin.bin", txn.data(), txn.size());
        unpatched_bytes = txn;
        unpatched_source = "get_transaction_bin";
        txn_sz = txn.size();
        txn_nz = nonzero_count(txn.data(), txn.size());
        txn_ok = true;
        classic_ok = true;
        std::cout << "  dumped instr_vendor_txn_bin.bin size=" << txn_sz
                  << " nonzero=" << txn_nz
                  << " label=UNPATCHED_get_transaction_bin_classic"
                  << " inputs={ti,tw}\n";
      } else if (prefer_as_instr_payload(txn) &&
                 is_oversized_multi_txn(txn.size())) {
        // HITL: get_transaction_bin can return multi-txn (~32k) with different
        // body than classic staged 1688 — never primary post_patch / txn_bin.
        write_bytes(base / "instr_vendor_get_transaction_bin.bin", txn.data(),
                    txn.size());
        multi_txn_sz = txn.size();
        std::cout << "  dumped instr_vendor_get_transaction_bin.bin size="
                  << multi_txn_sz << " nonzero="
                  << nonzero_count(txn.data(), txn.size())
                  << " label=DIAG_multi_txn_NOT_classic_primary"
                  << " (will prefer Transaction map 1688)\n";
        log_txn_hdr_diag("get_transaction_bin multi", txn.data(), txn.size());
        // Research negative: not ELF; not a container of classic 1688.
        if (txn.size() >= 4 && txn[0] == 0x7f && txn[1] == 'E' &&
            txn[2] == 'L' && txn[3] == 'F') {
          std::cout << "  multi is ELF — aiebu peel may apply (unexpected for "
                       "this key)\n";
        } else {
          std::cout << "  multi aiebu peel: N/A (not ELF; classic magic "
                       "layout)\n";
        }
      } else if (!txn.empty()) {
        std::cout << "  get_transaction_bin unusable size=" << txn.size()
                  << " (need classic " << kMinInstrBytes << " non-zero; "
                  << "will try Transaction map)\n";
        // Still stash short dump for diagnostics only (not txn_ok).
        write_bytes(base / "instr_vendor_txn_bin_short.bin", txn.data(),
                    txn.size());
      } else {
        std::cout << "  get_transaction_bin empty\n";
      }
    } catch (const std::exception &e) {
      std::cout << "  get_transaction_bin err: " << e.what() << "\n";
      std::cout << "    (if range_check size=1: ensure inputs={ti,tw} — "
                   "fixed path)\n";
    }

    // Candidate keys: staged + GM siblings known in libtransaction for this shape.
    const std::string dims = std::to_string(kM) + "_" + std::to_string(kK) +
                             "_" + std::to_string(kN) + "_" +
                             std::to_string(kGs);
    const std::string prefix_a16f =
        "mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_";
    std::vector<std::string> keys = {
        kStagedKey,
        std::string(kStagedKey) + "_ctrl",
        // GM variants present in libtransaction dumps for 1_1152_256_128
        std::string(kStagedKey) + "_1_4096_gm",
        std::string(kStagedKey) + "_1_4096_gm_ctrl",
        std::string(kStagedKey) + "_1_16384_gm",
        std::string(kStagedKey) + "_1_16384_gm_ctrl",
        std::string(kStagedKey) + "_1_32768_gm",
        std::string(kStagedKey) + "_1_32768_gm_ctrl",
        prefix_a16f + dims,
        std::string("mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16w4acc16f_") +
            dims,
        std::string("mladfmatmulbias_prmptflat_") + dims,
        std::string("mladfmatmulbias_") + dims,
        std::string("mladfmatmulbias_1_1152_256_128"),
        std::string("mladf_1_1152_256_128"),
        std::string("mladfmatmulbias_mladf_2x4x4_flat_a16fw4acc16f_") + dims,
        // OpInterface helper shape form (prefix + dims)
        op.get_instr_key(
            "mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f",
            {kM, kK, kN, kGs}),
        op.get_instr_key("mladfmatmulbias", {kM, kK, kN, kGs}),
    };
    // Also try Transaction match_prefix (cap: only primary keys with our dims).
    try {
      auto &txn_map = Transaction::getInstance();
      auto matched = txn_map.match_prefix(std::string(kStagedKey),
                                          /*use_mds_elf=*/false);
      for (const auto &m : matched) {
        if (is_primary_instr_key(m) &&
            m.find("1_1152_256_128") != std::string::npos) {
          keys.push_back(m);
        }
      }
      auto matched2 = txn_map.match_prefix(prefix_a16f + "1_1152_256",
                                           /*use_mds_elf=*/false);
      for (const auto &m : matched2) {
        if (is_primary_instr_key(m) &&
            m.find("1_1152_256_128") != std::string::npos) {
          keys.push_back(m);
        }
      }
    } catch (const std::exception &e) {
      std::cout << "  txn match_prefix err: " << e.what() << "\n";
    }
    dedup_keys(keys);

    // Primary-only keys for registry post_patch attempts (Issue 1/9).
    std::vector<std::string> primary_keys;
    primary_keys.reserve(keys.size());
    for (const auto &k : keys) {
      if (is_primary_instr_key(k)) {
        primary_keys.push_back(k);
      }
    }

    // --- 2) ALWAYS attempt Transaction map dump for staged key family ---
    // Prefer classic 1688 map even when get_transaction_bin returned multi-txn.
    // Map dump always overwrites instr_vendor_txn_bin.bin with classic when hit.
    std::ostringstream attempts;
    std::cout << "  Transaction map dump (always try staged primary keys):\n";
    {
      size_t map_sz = 0;
      size_t map_nz = 0;
      std::string map_key;
      std::vector<std::uint8_t> map_bytes;
      // Prefer primary_keys first (staged at front); include sibling list for
      // optional ctrl diagnostics only via dump_txn_container_keys skip path.
      std::vector<std::string> map_keys = primary_keys;
      // Optionally attach known _ctrl siblings for diag dumps only.
      map_keys.push_back(std::string(kStagedKey) + "_ctrl");
      if (dump_txn_container_keys(base, map_keys, &map_sz, &map_nz, &map_key,
                                  &attempts, &map_bytes)) {
        // Always adopt classic map as primary unpatched (over multi-txn).
        const bool map_is_classic =
            is_classic_instr_size(map_sz) || map_sz == 4096;
        if (map_is_classic) {
          if (!map_bytes.empty()) {
            unpatched_bytes = std::move(map_bytes);
          } else {
            try {
              unpatched_bytes =
                  Transaction::getInstance().get_txn_bvec(map_key);
            } catch (...) {
              std::ifstream in(base / "instr_vendor_txn_bin.bin",
                               std::ios::binary);
              if (in) {
                unpatched_bytes.assign(std::istreambuf_iterator<char>(in),
                                       std::istreambuf_iterator<char>());
              }
            }
          }
          unpatched_source = "txn_container";
          txn_sz = map_sz;
          txn_nz = map_nz;
          txn_ok = true;
          classic_ok = true;
          std::cout << "  ALWAYS dumped instr_vendor_txn_bin.bin size="
                    << txn_sz << " nonzero=" << txn_nz
                    << " label=UNPATCHED_txn_container_classic key=" << map_key
                    << (multi_txn_sz > 0
                            ? " (preferred over multi-txn get_transaction_bin)"
                            : "")
                    << "\n";
          if (!unpatched_bytes.empty()) {
            log_txn_hdr_diag("txn_container classic", unpatched_bytes.data(),
                             unpatched_bytes.size());
          }
          // If multi diag exists, prove classic is not embedded (peel N/A).
          if (multi_txn_sz > 0 && !unpatched_bytes.empty()) {
            try {
              std::ifstream min(base / "instr_vendor_get_transaction_bin.bin",
                                std::ios::binary);
              std::vector<std::uint8_t> multi(
                  (std::istreambuf_iterator<char>(min)),
                  std::istreambuf_iterator<char>());
              const bool emb = bytes_contain_slice(
                  multi.data(), multi.size(), unpatched_bytes.data(),
                  unpatched_bytes.size());
              std::cout << "  multi embeds classic1688="
                        << (emb ? "1" : "0")
                        << " (research: expect 0 — cannot peel multi→classic)\n";
            } catch (...) {
              // ignore
            }
          }
        } else if (!txn_ok) {
          // Non-classic map hit only if nothing better (should be rare).
          if (!map_bytes.empty()) {
            unpatched_bytes = std::move(map_bytes);
          }
          unpatched_source = "txn_container";
          txn_sz = map_sz;
          txn_nz = map_nz;
          txn_ok = prefer_as_instr_payload(unpatched_bytes);
          classic_ok = false;
          std::cout << "  txn_map primary (non-classic) size=" << map_sz
                    << " key=" << map_key << "\n";
        } else {
          std::cout << "  txn_map hit size=" << map_sz << " key=" << map_key
                    << " (kept existing classic primary)\n";
        }
      } else if (!txn_ok) {
        std::cout << "  Transaction map: no usable classic " << kMinInstrBytes
                  << "B primary txn for staged family\n";
      } else {
        std::cout << "  Transaction map: no classic primary hit (kept "
                     "get_transaction_bin classic)\n";
      }
    }

    // --- Optional ctrl packets (IFM+WTS same as get_transaction_bin) ---
    size_t ctrl_pkt_sz = 0;
    try {
      OpInterface &opi = op;
      auto pkts = opi.get_ctrl_pkts(inputs_txn, outputs, attr);
      if (!pkts.empty() && !bytes_all_zero(pkts.data(), pkts.size())) {
        write_bytes(base / "ctrl_vendor_pkts.bin", pkts.data(), pkts.size());
        ctrl_pkt_sz = pkts.size();
        std::cout << "  dumped ctrl_vendor_pkts.bin size=" << ctrl_pkt_sz
                  << "\n";
      } else {
        std::cout << "  get_ctrl_pkts empty or all-zero (skip)\n";
      }
    } catch (const std::exception &e) {
      std::cout << "  get_ctrl_pkts err: " << e.what() << "\n";
    }

    // --- 3) Registry post-patch BO (prefer) ---
    auto &reg = OpInterface::get_instr_reg();
    std::string used_key;
    size_t instr_sz = 0;
    size_t instr_nz = 0;
    bool dumped = false;
    bool from_registry = false;
    bool txn_fallback = false;
    std::string dump_note;
    std::string capture_mode = "none";

    std::cout << "  registry key attempts (" << primary_keys.size()
              << " primary; siblings skipped for post_patch):\n";
    // Always print capacity-fit research (HITL miss ≠ pure-direct size bug).
    log_instr_bo_limit_research("", kMinInstrBytes);
    bool instr_bo_err_logged = false;
    for (const auto &key : primary_keys) {
      // Issue 1/9: never write *_ctrl / *_ctrl_meta / *_param as post_patch.
      if (!is_primary_instr_key(key)) {
        continue;
      }
      try {
        const bool in_reg = reg.instr_in_registry(key);
        if (!in_reg) {
          std::cout << "    miss  instr_in_registry: " << key
                    << " (still try get_instr_bo)\n";
          attempts << "miss:" << key << ";";
          // Fall through: some builds disagree with instr_in_registry.
        } else {
          std::cout << "    hit   instr_in_registry: " << key << "\n";
        }
        xrt::bo ibo = reg.get_instr_bo(key);
        const size_t sz = ibo.size();
        if (sz == 0) {
          std::cout << "    empty BO for key: " << key << "\n";
          attempts << "empty:" << key << ";";
          continue;
        }
        void *mapped = ibo.map<void *>();
        if (mapped == nullptr) {
          std::cout << "    map failed for key: " << key << "\n";
          attempts << "mapfail:" << key << ";";
          continue;
        }
        const auto *p = static_cast<const std::uint8_t *>(mapped);
        if (bytes_all_zero(p, sz)) {
          std::cout << "    all-zero BO for key: " << key << " size=" << sz
                    << "\n";
          attempts << "zero:" << key << ";";
          continue;
        }
        // Prefer ≥ content size; accept classic 1688 or page-pad 4096.
        // Refuse oversized multi-txn class (e.g. 32k) as post_patch — wrong body
        // for staged key; dump diagnostic only and keep trying other keys.
        if (sz < kMinInstrBytes) {
          std::cout << "    too-small BO for key: " << key << " size=" << sz
                    << " (skip as post_patch)\n";
          attempts << "small:" << key << ";";
          continue;
        }
        if (is_oversized_multi_txn(sz)) {
          write_bytes(base / ("registry_multi_" + key + ".bin"), p, sz);
          std::cout << "    oversized multi-class BO for key: " << key
                    << " size=" << sz
                    << " (skip as post_patch — diag only; prefer classic "
                    << kMinInstrBytes << " or page-pad 4096)\n";
          attempts << "multi_skip:" << key << ";";
          continue;
        }
        instr_nz = nonzero_count(p, sz);
        write_bytes(base / "instr_vendor_post_patch.bin", p, sz);
        used_key = key;
        instr_sz = sz;
        dumped = true;
        from_registry = true;
        capture_mode = "registry_get_instr_bo";
        dump_note =
            "registry get_instr_bo = Transaction/cache host image (NOT proven "
            "live-patched run_ BO after execute); preferred over unpatched map "
            "when classic-sized";
        attempts << "DUMPED:" << key << ";";
        std::cout << "    DUMPED key=" << key << " size=" << sz
                  << " nonzero=" << instr_nz
                  << " (Transaction/cache image; not proven live run_ BO)"
                  << (in_reg ? "" : " (instr_in_registry was false)") << "\n";
        break;
      } catch (const std::exception &e) {
        const std::string emsg = e.what();
        std::cout << "    err   key=" << key << " : " << emsg << "\n";
        attempts << "err:" << key << ";";
        // Extra detail once when throw mentions INSTR_BO / failed buffer.
        if (!instr_bo_err_logged &&
            (emsg.find("INSTR_BO_LIMIT") != std::string::npos ||
             emsg.find("Instruction buffer size") != std::string::npos ||
             emsg.find("Failed to get instruction buffer") !=
                 std::string::npos)) {
          log_instr_bo_limit_research(emsg, kMinInstrBytes);
          instr_bo_err_logged = true;
        }
      }
    }

    // Optional: dump registry _ctrl under separate name (never post_patch).
    for (const auto &key : keys) {
      if (!is_sibling_suffix_key(key)) {
        continue;
      }
      if (key.size() < 5 || key.compare(key.size() - 5, 5, "_ctrl") != 0) {
        continue;
      }
      try {
        if (!reg.instr_in_registry(key)) {
          continue;
        }
        xrt::bo ibo = reg.get_instr_bo(key);
        const size_t sz = ibo.size();
        if (sz == 0) {
          continue;
        }
        void *mapped = ibo.map<void *>();
        if (mapped == nullptr) {
          continue;
        }
        const auto *p = static_cast<const std::uint8_t *>(mapped);
        if (bytes_all_zero(p, sz)) {
          continue;
        }
        write_bytes(base / ("registry_" + key + ".bin"), p, sz);
        std::cout << "    registry sibling dump: " << key << " size=" << sz
                  << " (not post_patch)\n";
        break; // one ctrl is enough for diagnostics
      } catch (...) {
        // ignore
      }
    }

    // --- 4) Fallback: classic unpatched txn as post_patch (honest modes) ---
    // Prefer classic 1688; never write multi-txn 32k as post_patch when classic
    // instr_vendor_txn_bin.bin exists (HITL false CLASS_HINT C root cause).
    auto write_unpatched_post_patch =
        [&](const std::vector<std::uint8_t> &bytes, const std::string &src,
            const std::string &mode, const std::string &note) {
          write_bytes(base / "instr_vendor_post_patch.bin", bytes.data(),
                      bytes.size());
          instr_sz = bytes.size();
          instr_nz = nonzero_count(bytes.data(), bytes.size());
          used_key = "(" + src + ")";
          dumped = true;
          txn_fallback = true;
          capture_mode = mode;
          dump_note = note;
          std::cout << "  FALLBACK → instr_vendor_post_patch.bin from " << src
                    << " size=" << instr_sz << " nonzero=" << instr_nz
                    << " capture_mode=" << capture_mode
                    << (classic_ok ? " classic=1" : " classic=0") << "\n";
          std::cout
              << "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\n"
              << "CAPTURE_MODE_UNPATCHED=" << capture_mode << "\n"
              << "  Filename instr_vendor_post_patch.bin is UNPATCHED fallback\n"
              << "  NOT proven live-patched run_ BO; NOT C-ready\n"
              << "  C only if agent says ready (registry/parity alone never unlock)\n"
              << "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\n";
        };

    if (!dumped && txn_ok && !unpatched_bytes.empty() &&
        prefer_as_classic_instr_payload(unpatched_bytes)) {
      const std::string mode = unpatched_source == "txn_container"
                                   ? "txn_container_unpatched"
                                   : "txn_bin_fallback_unpatched";
      const std::string note =
          unpatched_source == "txn_container"
              ? "FALLBACK Transaction::get_txn_bvec classic 1688 — UNPATCHED "
                "libtransaction map; NOT multi-txn get_transaction_bin body"
              : "FALLBACK get_transaction_bin classic 1688 — UNPATCHED; "
                "diff vs pure-direct reflects pre-patch content only";
      write_unpatched_post_patch(unpatched_bytes, unpatched_source, mode, note);
    } else if (!dumped && txn_ok) {
      // txn_ok but cache empty/non-classic — re-read classic txn file
      try {
        std::ifstream in(base / "instr_vendor_txn_bin.bin", std::ios::binary);
        std::vector<std::uint8_t> file_bytes(
            (std::istreambuf_iterator<char>(in)),
            std::istreambuf_iterator<char>());
        if (prefer_as_classic_instr_payload(file_bytes)) {
          classic_ok = true;
          const std::string mode = unpatched_source == "txn_container"
                                       ? "txn_container_unpatched"
                                       : "txn_bin_fallback_unpatched";
          write_unpatched_post_patch(
              file_bytes, "instr_vendor_txn_bin.bin", mode,
              "FALLBACK copy of classic instr_vendor_txn_bin.bin — UNPATCHED");
        } else if (prefer_as_instr_payload(file_bytes)) {
          // Last resort: non-classic primary only if no classic available.
          write_unpatched_post_patch(
              file_bytes, "instr_vendor_txn_bin.bin",
              "txn_bin_fallback_unpatched",
              "FALLBACK non-classic instr_vendor_txn_bin.bin — UNPATCHED "
              "(no classic 1688; multi-txn body may false CLASS_HINT C)");
        }
      } catch (const std::exception &e) {
        std::cout << "  txn file fallback: " << e.what() << "\n";
      }
    } else if (!dumped && multi_txn_sz > 0) {
      // No classic map and no classic get_transaction_bin — last-resort multi.
      try {
        std::ifstream in(base / "instr_vendor_get_transaction_bin.bin",
                         std::ios::binary);
        std::vector<std::uint8_t> multi(
            (std::istreambuf_iterator<char>(in)),
            std::istreambuf_iterator<char>());
        if (prefer_as_instr_payload(multi)) {
          // Also stash as txn_bin so tools have *something* named consistently.
          write_bytes(base / "instr_vendor_txn_bin.bin", multi.data(),
                      multi.size());
          txn_sz = multi.size();
          txn_nz = nonzero_count(multi.data(), multi.size());
          write_unpatched_post_patch(
              multi, "get_transaction_bin_multi", "txn_bin_fallback_unpatched",
              "FALLBACK multi-txn get_transaction_bin ONLY — no classic 1688; "
              "diff first-1688 vs pure is WRONG BODY (CLASS_HINT D, not C)");
        }
      } catch (const std::exception &e) {
        std::cout << "  multi-txn last-resort: " << e.what() << "\n";
      }
    }

    if (!dumped) {
      std::cerr
          << "ERROR: could not dump instruction BO "
             "(registry + get_transaction_bin + Transaction map)\n";
      std::cerr << "  OFM golden still written — NPU ran, but instr hook "
                   "failed\n";
      std::cerr << "  attempts: " << attempts.str() << "\n";
      print_l3_restore_reminder();
      return 1;
    }

    // Success gate: ≥1688 non-all-zero
    if (instr_sz < kMinInstrBytes || instr_nz == 0) {
      std::cerr << "ERROR: post_patch too small or all-zero size=" << instr_sz
                << " nonzero=" << instr_nz << " (need ≥" << kMinInstrBytes
                << " non-zero)\n";
      print_l3_restore_reminder();
      return 1;
    }

    const auto instr_path = base / "instr_vendor_post_patch.bin";
    const std::string sha = sha256_file(instr_path);
    const bool paddrs_known =
        ifm_paddr != 0 && wts_paddr != 0 && ofm_paddr != 0;

    // vendor_paddrs.json schema v1
    {
      std::ofstream j(base / "vendor_paddrs.json");
      j << "{\n"
        << "  \"schema_version\": 1,\n"
        << "  \"source\": \"capture_mladf_instr_vendor\",\n"
        << "  \"capture_mode\": \"" << json_escape(capture_mode) << "\",\n"
        << "  \"key\": \"" << json_escape(used_key) << "\",\n"
        << "  \"staged_key\": \"" << json_escape(kStagedKey) << "\",\n"
        << "  \"instr_bytes\": " << instr_sz << ",\n"
        << "  \"instr_nonzero_bytes\": " << instr_nz << ",\n"
        << "  \"instr_sha256\": \"" << sha << "\",\n"
        << "  \"txn_bin_bytes\": " << txn_sz << ",\n"
        << "  \"txn_bin_nonzero_bytes\": " << txn_nz << ",\n"
        << "  \"classic_unpatched\": " << (classic_ok ? "true" : "false")
        << ",\n"
        << "  \"multi_txn_get_transaction_bin_bytes\": " << multi_txn_sz
        << ",\n"
        << "  \"txn_bin_fallback_likely_unpatched\": "
        << (txn_fallback ? "true" : "false") << ",\n"
        << "  \"from_registry\": " << (from_registry ? "true" : "false")
        << ",\n"
        << "  \"unpatched_source\": \"" << json_escape(unpatched_source)
        << "\",\n"
        << "  \"ctrl_pkts_bytes\": " << ctrl_pkt_sz << ",\n"
        << "  \"ifm_paddr\": " << hex_or_null(ifm_paddr) << ",\n"
        << "  \"wts_paddr\": " << hex_or_null(wts_paddr) << ",\n"
        << "  \"ofm_paddr\": " << hex_or_null(ofm_paddr) << ",\n"
        << "  \"ctrl_paddr\": " << hex_or_null(ctrl_paddr) << ",\n"
        << "  \"paddrs_known\": " << (paddrs_known ? "true" : "false")
        << ",\n"
        << "  \"registry_attempts\": \"" << json_escape(attempts.str())
        << "\",\n"
        << "  \"instr_bo_align\": " << kInstrBoAlign << ",\n"
        << "  \"instr_bo_capacity_default\": " << kInstrBoCapacity << ",\n"
        << "  \"live_instr_bo_public_api\": false,\n"
        << "  \"note\": \"IO paddrs best-effort from xrt::bo::address after "
           "execute; null means re-patch pure-direct with synthetic paddrs "
           "or last_apply extract. capture_mode=registry_get_instr_bo is "
           "Transaction/cache host image only (NOT proven live-patched run_ "
           "BO). txn_*_unpatched = classic 1688 UNPATCHED. Multi get_transaction_bin "
           "diag only (wrong body; not ELF; classic1688 not embedded). "
           "get_transaction_bin needs inputs={IFM,WTS}. No public live instr BO. "
           "INSTR_BO_LIMIT = align_to_next(len+8, 0x8000) > capacity_ (default "
           "0x3c00000; 1688+32208 both fit). C only if agent says ready; "
           "unpatched A / registry / parity alone ≠ C-ready.\"\n"
        << "}\n";
    }

    {
      std::ofstream p(base / "PROVENANCE_VENDOR_INSTR.md");
      p << "# PROVENANCE — vendor instruction capture\n\n"
        << "**Not dragon-npu ran_on_npu.**\n\n"
        << "| Field | Value |\n"
        << "|-------|-------|\n"
        << "| capture_mode | `" << capture_mode << "` |\n"
        << "| Key / registry | `" << used_key << "` |\n"
        << "| Instr bytes | " << instr_sz << " |\n"
        << "| Non-zero bytes | " << instr_nz << " |\n"
        << "| sha256 | `" << sha << "` |\n"
        << "| txn bin classic (unpatched) | " << txn_sz << " B ("
        << unpatched_source << ") |\n"
        << "| multi-txn get_transaction_bin (diag) | " << multi_txn_sz
        << " B |\n"
        << "| classic_unpatched | " << (classic_ok ? "true" : "false") << " |\n"
        << "| ctrl pkts | " << ctrl_pkt_sz << " B |\n"
        << "| Dump note | " << dump_note << " |\n"
        << "| OFM golden | ofm_golden.bin (non-zero; NPU ran) |\n\n"
        << "## Honesty\n\n"
        << "- Prefer `capture_mode=registry_get_instr_bo` when available "
           "(Transaction/cache image — **not** proven live-patched `run_` BO).\n"
        << "- `txn_*_unpatched` modes: file name `instr_vendor_post_patch.bin` "
           "is **misleading** — content is classic 1688 UNPATCHED; "
           "**NOT C-ready** alone.\n"
        << "- **C only if agent says ready** — registry/unpatched A/parity alone "
           "never unlock C.\n"
        << "- `instr_vendor_txn_bin.bin` = best classic UNPATCHED 1688 when "
           "available (Transaction map preferred over multi-txn).\n"
        << "- `instr_vendor_get_transaction_bin.bin` = oversized multi-txn "
           "diagnostic only (different body — do not diff prefix as C).\n"
        << "- Sibling keys `*_ctrl` / `*_ctrl_meta` / `*_param` never used as "
           "post_patch.\n"
        << "- `get_transaction_bin` must pass IFM+WTS (`inputs={ti,tw}`) — "
           "IFM-only hits fit_MKN range_check.\n"
        << "- **INSTR_BO_LIMIT** (DD `instruction_cache::put`): "
           "`align_to_next(txn_len+8, 0x8000) > capacity_` "
           "(default capacity `0x3c00000` / 60MiB). Not a pure-direct bug.\n"
        << "- **No live instr BO public API** on `mladfmatmulbias` after "
           "execute (`run_` private; only IFM/WTS/OFM/const/ctrl). "
           "`get_instr_bo` loads Transaction→cache only.\n"
        << "- Multi ~32208: classic-format wrong body (500 ops); "
           "`find(classic1688)==-1`; not ELF — no aiebu peel.\n"
        << "- Unpatched CLASS A / mere `diff rc=0` **≠ C-ready**.\n"
        << "- Diff: `bash tools/diff_vendor_instr_parity.sh`\n"
        << "- Restore L3: `sudo sh -c 'echo 0 > "
           "/sys/module/amdxdna/parameters/exec_cmd_allow'`\n";
    }

    {
      std::ofstream m(base / "MANIFEST.vendor_instr.sha256");
      m << sha << "  instr_vendor_post_patch.bin\n";
      if (txn_sz > 0 && fs::exists(base / "instr_vendor_txn_bin.bin")) {
        m << sha256_file(base / "instr_vendor_txn_bin.bin")
          << "  instr_vendor_txn_bin.bin\n";
      }
      if (multi_txn_sz > 0 &&
          fs::exists(base / "instr_vendor_get_transaction_bin.bin")) {
        m << sha256_file(base / "instr_vendor_get_transaction_bin.bin")
          << "  instr_vendor_get_transaction_bin.bin\n";
      }
    }

    std::cout << "capture_mladf_instr_vendor → " << fs::absolute(base) << "\n"
              << "  capture_mode=" << capture_mode << "\n"
              << "  instr_vendor_post_patch.bin  " << instr_sz << " B  sha="
              << sha << (classic_ok && txn_fallback ? " classic_unpatched" : "")
              << "\n"
              << "  ofm_golden.bin written (NPU OK)\n";
    if (multi_txn_sz > 0) {
      std::cout << "  multi-txn diagnostic: "
                   "instr_vendor_get_transaction_bin.bin "
                << multi_txn_sz << " B (not primary)\n";
    }
    if (txn_fallback || capture_mode.find("unpatched") != std::string::npos) {
      std::cout
          << "CAPTURE_MODE_UNPATCHED=" << capture_mode
          << " — unpatched ≠ live-patched run_ BO; NOT C-ready "
             "(C only if agent says ready; registry/parity alone never unlock)\n";
    }
    std::cout << "\nNext (offline):\n"
              << "  bash tools/diff_vendor_instr_parity.sh\n"
              << "Restore L3 if opened:\n"
              << "  sudo sh -c 'echo 0 > "
                 "/sys/module/amdxdna/parameters/exec_cmd_allow'\n";
    return 0;
  } catch (const std::exception &e) {
    std::cerr << "ERROR: " << e.what() << "\n"
              << "  Temporary L3 if hardened:\n"
              << "    sudo sh -c 'echo 1 > "
                 "/sys/module/amdxdna/parameters/exec_cmd_allow'\n";
    print_l3_restore_reminder();
    return 1;
  }
}
