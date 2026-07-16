# Check fill_slot on latest mainline in-tree amdxdna

Goal (after amd/xdna-driver PR #1493 closed): see if the 2026-07-13 lab failure
still exists on **torvalds/linux** in-tree `drivers/accel/amdxdna`, as maxzhen
suggested.

## Static findings (2026-07-16, no reboot)

Fetched `drivers/accel/amdxdna/aie2_message.c` from **torvalds/linux master**.

### 1. Mainline no longer uses `cmd_abo->client->xdna` in fill_slot

**Mainline (master):**

```c
aie2_cmdlist_fill_slot(...)
{
    struct amdxdna_dev *xdna = to_xdna_dev(to_gobj(cmd_abo)->dev);
    ...
}
```

**Lab zen 7.1 / xdna-driver staging (still, as of our tree):**

```c
struct amdxdna_dev *xdna = cmd_abo->client->xdna;
```

So the exact NULL pattern we attributed to `client` in the oops is already
**rewritten** on mainline: device is taken from the GEM object’s `dev`, not
`cmd_abo->client`.

### 2. Incomplete START_NPU payload still rejected with -EINVAL

Same class of check maxzhen cited, present on mainline
`aie2_cmdlist_fill_npu_dpu`:

```c
sn = amdxdna_cmd_get_payload(cmd_bo, &cmd_len);
arg_sz = cmd_len - sizeof(*sn);
if (cmd_len < sizeof(*sn) || arg_sz > MAX_NPU_ARGS_SIZE)
    return -EINVAL;
```

Our repro’s short/incomplete CMD BO should hit **errno**, not oops, on current
mainline *if* it reaches this fill helper with a live `cmd_abo`.

### 3. Host context

| Item | Value |
|------|--------|
| Running kernel | `7.1.3-zen1-2-zen` |
| Loaded amdxdna | Lab **updates/** hardened OOT (`exec_cmd_allow`, srcversion E36F…) |
| Stock in-tree .ko | Still under `.../kernel/drivers/accel/amdxdna/amdxdna.ko.zst` |
| Disk | Plenty for a full kernel build if needed |

## Test ladder (cheapest first)

### A. Static only (done)

Mainline already differs; likely **not** the same client-NULL path.

### B. Stock **zen 7.1** in-tree (same generation as the oops)

Does **not** prove mainline; does re-check “is stock 7.1 still bad without lab module?”

```bash
# DANGER: repro can oops/wedge on unpatched stock
bash scripts/diagnose_amdxdna.sh   # must be clean

# unload lab module; load distro in-tree
sudo modprobe -r amdxdna
sudo modprobe amdxdna
modinfo amdxdna | grep -E 'filename|srcversion|exec_cmd'
# expect: kernel/drivers/accel/...  and NO exec_cmd_allow

gcc -O2 -Wall -Wextra -o /tmp/repro examples/repro_amdxdna_cmdlist_oops.c
# ONLY if you accept wedge risk:
/tmp/repro --i-know-this-can-wedge

journalctl -k -b --since '5 min ago' | grep -Ei 'fill_slot|Oops|amdxdna'

# restore lab module when done (if still unloadable):
# sudo modprobe -r amdxdna
# sudo insmod /lib/modules/$(uname -r)/updates/amdxdna-hardened/amdxdna.ko
# or reboot
```

### C. **Latest mainline** in-tree (what maxzhen asked for)

Building only `amdxdna.ko` against **running zen headers** from mainline
sources is **unreliable** (API drift). Honest options:

#### C1. Full mainline (or recent stable) kernel build + boot (correct, heavy)

```bash
# shallow clone
git clone --depth=1 https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git \
  ~/Projects/kernel/linux-mainline
cd ~/Projects/kernel/linux-mainline

# start from running config
zcat /proc/config.gz > .config   # or: cp /boot/config-$(uname -r) .config
# ensure accel + amdxdna
scripts/config --enable CONFIG_DRM_ACCEL
scripts/config --module CONFIG_DRM_ACCEL_AMDXDNA   # name may vary; check Kconfig
make olddefconfig
# confirm:
grep AMDXDNA .config

make -j"$(nproc)" bzImage modules
# install is root + reboot; only when you choose:
# sudo make modules_install install
# then reboot into new kernel and run repro
```

On Garuda, also keep a known-good boot entry (zen) before replacing kernels.

#### C2. Wait for Arch `linux` package to pick up the `to_xdna_dev` fill_slot

Arch already has `linux 7.1.3.arch1-2` (same major as zen). That may or may not
include the mainline rewrite. Diffing package sources vs torvalds is smaller than
a full custom build.

```bash
# optional: install vanilla linux package alongside zen (does not remove zen)
# sudo pacman -S linux linux-headers
# reboot, pick "linux" in boot menu, run repro on stock in-tree
```

### D. Document outcome

| Result | Conclusion |
|--------|------------|
| Mainline/static: no client deref + -EINVAL on short payload | Aligns with maxzhen; 7.1 zen was behind |
| Stock 7.1 zen still oopses | Bug was real on that tree; fixed later |
| Latest mainline still oopses | New report + patch to **kernel** list / maintainers |

## Dual check (zen + mainline) — reward careful review

You daily-drive **linux-zen**. Check **both**:

| Track | What | Answers |
|-------|------|---------|
| **A. Stock zen in-tree** | Unload lab module; load distro `amdxdna` on `7.1.3-zen*` | “Is *our* packaged zen still the bad generation?” |
| **B. Mainline** | Static (done) + optional boot of newer/vanilla tree | “Is tip-of-tree still vulnerable the way Max asked?” |

Helper (safe by default; does not open accel unless `--run-repro`):

```bash
bash scripts/check_amdxdna_zen_and_mainline.sh
# after you switch to stock in-tree yourself:
bash scripts/check_amdxdna_zen_and_mainline.sh --run-repro
```

### Track A results (fill in)

| Step | Result |
|------|--------|
| Date / `uname -r` | |
| Lab module unloaded? | |
| `modinfo` filename (stock path?) | |
| Repro exit / errno | |
| journal fill_slot / Oops? | |
| Lab module restored? | |

### Track B results (fill in)

| Step | Result |
|------|--------|
| Mainline HEAD (sparse clone) | e.g. `58717b2` (refresh with git) |
| Static: `to_xdna_dev` in fill_slot? | **yes** on master as of 2026-07-16 |
| Static: short payload `-EINVAL`? | **yes** |
| Booted mainline/vanilla and ran repro? | (optional) |
| Empirical: oops or errno? | |

### Recommendation

1. **Track B static is already strong** for Max’s “latest source” question.
2. **Track A** once if you accept wedge risk on stock zen (same gen as original lab).
3. **Track B boot** only if you want belt-and-suspenders after A.

## Safety

- Never run repro while wedged; diagnose first.
- Prefer keeping **zen** as a known-good boot entry if you install another kernel.
- Lab `updates/` module is **not** stock zen and **not** mainline; unload it before claiming “in-tree” results.
- Do not run `--run-repro` while lab `exec_cmd_allow=0` module is loaded if you only want stock behavior (you will get -EPERM / different path).
