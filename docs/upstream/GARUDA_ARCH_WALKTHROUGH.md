# Step-by-step: report linux-zen amdxdna lag (Garuda user)

You drive **Garuda** + **linux-zen**. Process = **Arch package bug** (primary)
+ optional **Garuda forum** (secondary). You post; this repo only holds evidence.

**Evidence folder:** `docs/upstream/evidence/garuda-arch/`  
**Fact sheet:** `docs/upstream/evidence/garuda-arch/FACTSHEET.md`

---

## Phase 0 — local prep (can be done offline)

- [x] Fact sheet + oops attachments prepared in-repo  
- [ ] Optional: hide `updates/`, load stock zen, run repro once (wedge risk)  
- [ ] Optional: save `garuda-inxi` or `uname -a` paste for the report  

```bash
uname -a
pacman -Qi linux-zen | grep -E 'Version|Packager|Build Date'
modinfo /lib/modules/$(uname -r)/kernel/drivers/accel/amdxdna/amdxdna.ko.zst | head -20
```

---

## Phase 1 — Arch account + search (you do this in browser)

### Step 1.1 — Open Arch bugs

URL: https://bugs.archlinux.org/

### Step 1.2 — Log in / register

- Use an account you control (not shared).
- Confirm email if required.

### Step 1.3 — Search for duplicates

1. Search box: `linux-zen amdxdna`  
2. Also try: `amdxdna`, `1022:17f0`, `fill_slot`  
3. If an open task exists, **add a comment** with your version + oops instead of a new task.

### Step 1.4 — Tell the agent what you found

- "No existing task" or paste the task ID/link.

---

## Phase 2 — File Arch bug for package `linux-zen`

### Step 2.1 — New task

- Project: Arch Linux (default)  
- Category / package: **`linux-zen`** (type carefully)  
- Severity: often **Low** or **Medium** (not critical unless you argue NPU is critical for many users)  
- Priority: leave default unless you know Arch norms  

### Step 2.2 — Title (you choose; examples only)

Examples of shape (not scripts for you to parrot):

- `[linux-zen] amdxdna aie2_cmdlist_fill_slot matches NULL deref on incomplete EXEC_CMD (7.1.3)`  
- `[linux-zen] stock amdxdna fill_slot still uses client-indirect load; mainline uses to_xdna_dev`

### Step 2.3 — Description

Write in your voice. Pull **facts** from `FACTSHEET.md`. Suggested structure:

1. Summary (one short paragraph)  
2. Package + `uname -r`  
3. Hardware (PCI ID)  
4. Symptoms / oops (attach file)  
5. Stock module path + srcversion  
6. Disasm matches oops (attach or paste short snippet)  
7. Mainline / amd contrast (short)  
8. What you hope for (track merge / next zen)  

### Step 2.4 — Attachments

Upload from:

```text
docs/upstream/evidence/ATTACH_fill_slot_oops_20260713_220051.txt
docs/upstream/evidence/garuda-arch/zen-stock-fill_slot-disasm.txt
```

Optional: second oops, FACTSHEET.md as text.

### Step 2.5 — Submit

- Double-check package name is `linux-zen`  
- Submit  
- Save the **FS#…** number  

### Step 2.6 — After submit

- Paste the bug URL into chat if you want help on a follow-up comment  
- Do **not** expect instant replies  

---

## Phase 3 — Garuda forum (optional, after Arch bug exists)

### Step 3.1 — Forum

https://forum.garudalinux.org/

### Step 3.2 — Category

Usually **Issues & Assistance** or hardware-related (pick what fits current forum layout).

### Step 3.3 — Post shape

- You are on Garuda + linux-zen + Ryzen AI NPU  
- Documented oops; **Arch bug FS#…**  
- Not asking for a custom Garuda kernel  
- Point others to the Arch task; invite “me too” with package version  

### Step 3.4 — garuda-inxi

Forum often wants:

```bash
garuda-inxi
```

Paste output (you can redact serials).

---

## Phase 4 — After a new linux-zen lands

```bash
pacman -Qi linux-zen | grep Version
# reboot into new zen if needed
# temporarily hide updates/ and load stock — see dual-track script
bash scripts/check_amdxdna_zen_and_mainline.sh
```

If stock no longer matches old disasm / repro is clean errno only: drop daily reliance on lab `updates/` module.

---

## What the agent will / will not do

| Agent can | Agent will not |
|-----------|----------------|
| Build evidence packs, disasm, checklists | Post as you on Arch/Garuda/GitHub |
| Walk one step at a time in chat | Invent account passwords |
| Review *your* draft text for facts | Speak in your name to maintainers |

---

## Current checkpoint

**You are here:** Phase 0 mostly done → **Phase 1 Step 1.1** (open Arch bugs, search).

When ready, open https://bugs.archlinux.org/ and search. Reply with what you find (none / link), then we do Phase 2 step by step.
