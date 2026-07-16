// Dump DynamicDispatch Transaction map entries (lab only).
// Links the AMD-shipped libtransaction / DD .so that already contains
// prebuilt matmul txn binaries for STX.
//
// Build + run: see tools/dump_dd_txn.sh
//
// For each primary matmul key, also tries sibling keys:
//   <key>_ctrl_meta  (usually JSON buffer sizes)
//   <key>_ctrl       (binary control packet, optional)
//
// Output is AMD-licensed binary data — keep gitignored; do not public-CDN.

#include "txn/txn_container.hpp"

#include <cctype>
#include <cstdint>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include <system_error>
#include <vector>

namespace fs = std::filesystem;

static bool looks_like_json(const std::vector<std::uint8_t> &v) {
  size_t i = 0;
  while (i < v.size() &&
         (v[i] == ' ' || v[i] == '\t' || v[i] == '\n' || v[i] == '\r')) {
    ++i;
  }
  if (i >= v.size()) {
    return false;
  }
  if (v[i] != '{' && v[i] != '[') {
    return false;
  }
  // Require mostly printable ASCII for a short sample (meta is text JSON).
  size_t sample = std::min(v.size(), size_t(256));
  size_t printable = 0;
  for (size_t j = 0; j < sample; ++j) {
    unsigned char c = v[j];
    if (c == '\n' || c == '\r' || c == '\t' || (c >= 32 && c < 127)) {
      ++printable;
    }
  }
  return printable * 10 >= sample * 9;
}

static std::string sanitize_fname(std::string fname) {
  for (char &c : fname) {
    if (!(std::isalnum(static_cast<unsigned char>(c)) || c == '_' || c == '-' ||
          c == '.')) {
      c = '_';
    }
  }
  if (fname.size() > 180) {
    fname = fname.substr(0, 180);
  }
  return fname;
}

static void write_bytes(const std::string &path, const std::vector<std::uint8_t> &v) {
  std::ofstream f(path, std::ios::binary);
  if (!f) {
    throw std::runtime_error("open failed: " + path);
  }
  f.write(reinterpret_cast<const char *>(v.data()),
          static_cast<std::streamsize>(v.size()));
  f.flush();
  if (!f) {
    throw std::runtime_error("write failed: " + path);
  }
  f.close();
  if (f.fail()) {
    throw std::runtime_error("close failed: " + path);
  }
}

// Returns true if a non-empty payload was written.
static bool dump_key_payload(Transaction &txn, const std::string &key,
                             const std::string &out_dir, bool is_sibling) {
  std::vector<std::uint8_t> bytes;
  try {
    bytes = txn.get_txn_bvec(key);
  } catch (const std::exception &e) {
    if (!is_sibling) {
      std::cout << "  fail " << key << ": " << e.what() << "\n";
    }
    return false;
  }
  if (bytes.empty()) {
    if (!is_sibling) {
      std::cout << "  empty: " << key << "\n";
    }
    return false;
  }

  const bool as_json = looks_like_json(bytes);
  const std::string fname = sanitize_fname(key);
  const std::string ext = as_json ? ".json" : ".bin";
  const std::string path = out_dir + "/" + fname + ext;
  try {
    write_bytes(path, bytes);
  } catch (const std::exception &e) {
    std::cout << "  fail write " << key << ": " << e.what() << "\n";
    return false;
  }

  const char *tag = is_sibling ? "sibling" : "wrote";
  std::cout << "  " << tag << " " << bytes.size() << " B  " << path;
  if (as_json) {
    std::cout << "  (json)";
  }
  std::cout << "\n";

  // Peek classic txn header only for non-JSON primary dumps.
  if (!as_json && !is_sibling && bytes.size() >= 16) {
    std::cout << "    hdr maj=" << int(bytes[0]) << " min=" << int(bytes[1])
              << " gen=" << int(bytes[2]) << " rows=" << int(bytes[3])
              << " cols=" << int(bytes[4]) << "\n";
  }
  return true;
}

// Primary matmul keys are not already _ctrl / _ctrl_meta / _param siblings.
// Note: `_gm_ctrl_meta` already ends with `_ctrl_meta`, so no separate case.
static bool is_primary_matmul_key(const std::string &key) {
  auto ends_with = [&](const char *suf) {
    const size_t n = std::char_traits<char>::length(suf);
    return key.size() >= n && key.compare(key.size() - n, n, suf) == 0;
  };
  if (ends_with("_ctrl_meta") || ends_with("_ctrl") || ends_with("_param")) {
    return false;
  }
  return true;
}

static void dump_siblings(Transaction &txn, const std::string &key,
                          const std::string &out_dir) {
  // Prefer exact sibling names used by AMD DD maps.
  const std::string meta_key = key + "_ctrl_meta";
  const std::string ctrl_key = key + "_ctrl";
  dump_key_payload(txn, meta_key, out_dir, /*is_sibling=*/true);
  dump_key_payload(txn, ctrl_key, out_dir, /*is_sibling=*/true);
}

int main(int argc, char **argv) {
  const std::string prefix = (argc > 1) ? argv[1] : "mladfmatmul";
  const std::string out_dir = (argc > 2) ? argv[2] : "/tmp/dd_txn_dump";
  const int max_dump = (argc > 3) ? std::atoi(argv[3]) : 20;

  {
    std::error_code ec;
    fs::create_directories(out_dir, ec);
    if (ec) {
      std::cerr << "mkdir failed: " << out_dir << ": " << ec.message() << "\n";
      return 1;
    }
  }

  try {
    auto &txn = Transaction::getInstance();
    auto keys = txn.match_prefix(prefix, /*use_mds_elf=*/false);
    std::cout << "prefix=" << prefix << " matches=" << keys.size() << "\n";

    // If prefix is an exact full key, dump that key first (even if match_prefix
    // returns many longer keys that share the same prefix).
    int dumped = 0;
    bool exact_dumped = false;
    try {
      auto exact = txn.get_txn_bvec(prefix);
      if (!exact.empty()) {
        if (dump_key_payload(txn, prefix, out_dir, /*is_sibling=*/false)) {
          ++dumped;
          exact_dumped = true;
          if (is_primary_matmul_key(prefix)) {
            dump_siblings(txn, prefix, out_dir);
          }
        }
      }
    } catch (...) {
      // not an exact key — fall through to prefix matches
    }

    for (const auto &key : keys) {
      if (dumped >= max_dump) {
        break;
      }
      if (exact_dumped && key == prefix) {
        continue;
      }
      // Siblings are attached when their primary key is dumped; do not burn
      // max_dump budget on *_ctrl / *_ctrl_meta / *_param alone.
      if (!is_primary_matmul_key(key)) {
        continue;
      }
      try {
        if (!dump_key_payload(txn, key, out_dir, /*is_sibling=*/false)) {
          continue;
        }
        ++dumped;
        dump_siblings(txn, key, out_dir);
      } catch (const std::exception &e) {
        std::cout << "  fail " << key << ": " << e.what() << "\n";
      }
    }
    std::cout << "dumped=" << dumped << " primary keys -> " << out_dir << "\n";
    std::cout << "(siblings _ctrl_meta / _ctrl do not count against max)\n";
    return dumped > 0 ? 0 : 2;
  } catch (const std::exception &e) {
    std::cerr << "fatal: " << e.what() << "\n";
    return 1;
  }
}
