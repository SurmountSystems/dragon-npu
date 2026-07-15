/*
 * repro_amdxdna_cmdlist_oops.c
 *
 * Minimal userspace sequence that, on 2026-07-13 lab (PCI 1022:17f0,
 * in-tree amdxdna / zen 7.1.3), triggered:
 *
 *   BUG: kernel NULL pointer dereference, address: 0000000000000050
 *   RIP: aie2_cmdlist_fill_slot+0x26/0x130 [amdxdna]
 *   Workqueue: amdxdna_js drm_sched_run_job_work
 *   Call chain: aie2_cmdlist_single_execbuf → aie2_sched_job_run
 *   RAX=0, CR2=0x50
 *
 * Sequence (same class as dragon-npu scaffold bring-up):
 *   open /dev/accel/accel0
 *   CREATE_BO DEV_HEAP (if needed)
 *   CREATE_BO SHMEM UMQ + LOG
 *   CREATE_HWCTX
 *   CREATE_BO CMD (zeros / incomplete ERT header)
 *   DRM_IOCTL_AMDXDNA_EXEC_CMD  (SUBMIT_EXEC_BUF, cmd_count=1)
 *
 * ╔══════════════════════════════════════════════════════════════════╗
 * ║  DANGER: On an UNPATCHED driver this can OOPS and leave opens  ║
 * ║  in D-state (module refcnt wedge → reboot).                    ║
 * ║  Do NOT run on a production laptop without a recovery plan.    ║
 * ║  Requires explicit --i-know-this-can-wedge                     ║
 * ╚══════════════════════════════════════════════════════════════════╝
 *
 * Build:
 *   gcc -O2 -Wall -Wextra -o repro_amdxdna_cmdlist_oops \
 *       examples/repro_amdxdna_cmdlist_oops.c
 *
 * Run (must opt in):
 *   ./repro_amdxdna_cmdlist_oops --i-know-this-can-wedge
 *
 * Expected outcomes:
 *   - Unpatched: may oops (check dmesg/journal) or return ioctl error
 *   - L2 null-guards: ioctl should fail with errno, not oops
 *   - L3 exec_cmd_allow=0: EXEC_CMD → -EPERM before fill_slot
 *
 * Note for maintainers: this does not construct a "closed handle with
 * live BO" race. It reproduces the *lab* incomplete-cmdlist EXEC_CMD
 * path that hit fill_slot. Whether cmd_abo->client was NULL vs another
 * NULL base at that instruction is best confirmed under a debugger /
 * your tree; the attachable oops shows RAX=0 after a load from the
 * cmd BO object.
 */

#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <inttypes.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <unistd.h>

#include <drm/amdxdna_accel.h>
#include <drm/drm.h>

/* From linux/drm.h — base IOCTL helpers if not pulled in */
#ifndef DRM_IOCTL_BASE
#define DRM_IOCTL_BASE 'd'
#endif

#define DRM_IOCTL_AMDXDNA(nr, type) \
	DRM_IOWR(DRM_COMMAND_BASE + (nr), type)

static const char *dev_path = "/dev/accel/accel0";

/* Prefer header enum if present; fall back to documented order. */
#ifndef DRM_AMDXDNA_CREATE_HWCTX
#define DRM_AMDXDNA_CREATE_HWCTX 0
#endif

static int xioctl(int fd, unsigned long req, void *arg, const char *what)
{
	int r = ioctl(fd, req, arg);
	if (r < 0) {
		fprintf(stderr, "FAIL %s: %s (errno=%d)\n", what, strerror(errno),
			errno);
		return -1;
	}
	printf("OK   %s\n", what);
	return 0;
}

static int create_bo(int fd, uint32_t type, uint64_t size, uint32_t *out_handle)
{
	struct amdxdna_drm_create_bo bo = {
		.flags = 0,
		.vaddr = 0,
		.size = size,
		.type = type,
		.handle = 0,
	};
	unsigned long req =
		DRM_IOCTL_AMDXDNA(DRM_AMDXDNA_CREATE_BO, struct amdxdna_drm_create_bo);
	if (xioctl(fd, req, &bo, "CREATE_BO") < 0)
		return -1;
	*out_handle = bo.handle;
	printf("     CREATE_BO type=%u size=%" PRIu64 " handle=%u\n", type, size,
	       bo.handle);
	return 0;
}

/*
 * Pack a minimal amdxdna_cmd.header dword (little-endian layout used by
 * the driver FIELD_GET masks):
 *   state 4b | extra_cu 2b@10 | count 11b@12 | opcode 5b@23
 * ERT_START_NPU = 20 → incomplete / empty payload after header is intentional.
 */
static uint32_t pack_cmd_header(uint32_t state, uint32_t count, uint32_t opcode)
{
	return (state & 0xf) | ((count & 0x7ff) << 12) | ((opcode & 0x1f) << 23);
}

static int map_and_fill_cmd_bo(int fd, uint32_t handle, size_t size, bool fill_header)
{
	struct amdxdna_drm_get_bo_info info = {
		.ext = 0,
		.ext_flags = 0,
		.handle = handle,
		.pad = 0,
	};
	unsigned long req =
		DRM_IOCTL_AMDXDNA(DRM_AMDXDNA_GET_BO_INFO, struct amdxdna_drm_get_bo_info);
	if (xioctl(fd, req, &info, "GET_BO_INFO (cmd)") < 0)
		return -1;

	if (info.map_offset == UINT64_MAX || info.map_offset == 0) {
		/* Some paths return vaddr already; try mmap via map_offset only. */
		fprintf(stderr,
			"WARN GET_BO_INFO map_offset=%" PRIu64 " vaddr=%" PRIu64
			" — may not fill header\n",
			(uint64_t)info.map_offset, (uint64_t)info.vaddr);
		if (!fill_header)
			return 0;
	}

	if (!fill_header)
		return 0;

	void *p = mmap(NULL, size, PROT_READ | PROT_WRITE, MAP_SHARED, fd,
		       (off_t)info.map_offset);
	if (p == MAP_FAILED) {
		fprintf(stderr, "WARN mmap cmd BO failed: %s — leaving BO zeroed\n",
			strerror(errno));
		return 0; /* still submit zeros; that was enough in lab */
	}

	/* Zero entire BO, then write incomplete START_NPU-style header. */
	memset(p, 0, size);
	uint32_t hdr = pack_cmd_header(1 /* NEW */, 0 /* count */, 20 /* START_NPU */);
	memcpy(p, &hdr, sizeof(hdr));
	munmap(p, size);

	struct amdxdna_drm_sync_bo sync = {
		.handle = handle,
		.direction = SYNC_DIRECT_TO_DEVICE,
		.offset = 0,
		.size = size,
	};
	req = DRM_IOCTL_AMDXDNA(DRM_AMDXDNA_SYNC_BO, struct amdxdna_drm_sync_bo);
	(void)xioctl(fd, req, &sync, "SYNC_BO TO_DEVICE (cmd)");
	return 0;
}

static void usage(const char *argv0)
{
	fprintf(stderr,
		"Usage: %s --i-know-this-can-wedge [options]\n"
		"\n"
		"  --dev PATH          accel node (default %s)\n"
		"  --zero-cmd          leave CMD BO all-zero (default: write incomplete header)\n"
		"  --no-heap           skip DEV_HEAP create (if already present for process)\n"
		"  --umq-size N        UMQ BO size (default 16384)\n"
		"  --log-size N        log BO size (default 131072)\n"
		"  --cmd-size N        CMD BO size (default 4096)\n"
		"  --tiles N           CREATE_HWCTX num_tiles (default 4)\n"
		"  --mem-size N        CREATE_HWCTX mem_size (default 262144)\n"
		"\n"
		"Refuses to run without --i-know-this-can-wedge.\n",
		argv0, dev_path);
}

int main(int argc, char **argv)
{
	bool allow_wedge = false;
	bool fill_header = true;
	bool make_heap = true;
	uint64_t umq_size = 16384;
	uint64_t log_size = 131072;
	uint64_t cmd_size = 4096;
	uint32_t num_tiles = 4;
	uint32_t mem_size = 262144;

	for (int i = 1; i < argc; i++) {
		if (!strcmp(argv[i], "--i-know-this-can-wedge")) {
			allow_wedge = true;
		} else if (!strcmp(argv[i], "--zero-cmd")) {
			fill_header = false;
		} else if (!strcmp(argv[i], "--no-heap")) {
			make_heap = false;
		} else if (!strcmp(argv[i], "--dev") && i + 1 < argc) {
			dev_path = argv[++i];
		} else if (!strcmp(argv[i], "--umq-size") && i + 1 < argc) {
			umq_size = strtoull(argv[++i], NULL, 0);
		} else if (!strcmp(argv[i], "--log-size") && i + 1 < argc) {
			log_size = strtoull(argv[++i], NULL, 0);
		} else if (!strcmp(argv[i], "--cmd-size") && i + 1 < argc) {
			cmd_size = strtoull(argv[++i], NULL, 0);
		} else if (!strcmp(argv[i], "--tiles") && i + 1 < argc) {
			num_tiles = (uint32_t)strtoul(argv[++i], NULL, 0);
		} else if (!strcmp(argv[i], "--mem-size") && i + 1 < argc) {
			mem_size = (uint32_t)strtoul(argv[++i], NULL, 0);
		} else if (!strcmp(argv[i], "-h") || !strcmp(argv[i], "--help")) {
			usage(argv[0]);
			return 0;
		} else {
			fprintf(stderr, "Unknown arg: %s\n", argv[i]);
			usage(argv[0]);
			return 2;
		}
	}

	if (!allow_wedge) {
		usage(argv[0]);
		fprintf(stderr,
			"\nRefusing: pass --i-know-this-can-wedge after reading the DANGER banner.\n");
		return 2;
	}

	printf("=== amdxdna cmdlist / EXEC_CMD lab repro ===\n");
	printf("device: %s\n", dev_path);
	printf("WARNING: unpatched amdxdna may OOPS in aie2_cmdlist_fill_slot\n\n");

	int fd = open(dev_path, O_RDWR | O_CLOEXEC);
	if (fd < 0) {
		fprintf(stderr, "open(%s): %s\n", dev_path, strerror(errno));
		return 1;
	}
	printf("OK   open %s (fd=%d)\n", dev_path, fd);

	/* DEV_HEAP often required once per process (64 MiB alignment on lab). */
	if (make_heap) {
		uint32_t heap = 0;
		const uint64_t heap_sz = 64ULL * 1024 * 1024;
		if (create_bo(fd, AMDXDNA_BO_DEV_HEAP, heap_sz, &heap) < 0) {
			fprintf(stderr,
				"NOTE: DEV_HEAP create failed (maybe already exists); continuing\n");
		}
	}

	uint32_t umq = 0, logb = 0, cmd = 0;
	if (create_bo(fd, AMDXDNA_BO_SHMEM, umq_size, &umq) < 0)
		goto out_close;
	if (create_bo(fd, AMDXDNA_BO_SHMEM, log_size, &logb) < 0)
		goto out_close;

	struct amdxdna_qos_info qos = {
		.gops = 0,
		.fps = 0,
		.dma_bandwidth = 0,
		.latency = 0,
		.frame_exec_time = 0,
		.priority = AMDXDNA_QOS_NORMAL_PRIORITY,
	};

	struct amdxdna_drm_create_hwctx hw = {
		.ext = 0,
		.ext_flags = 0,
		.qos_p = (uint64_t)(uintptr_t)&qos,
		.umq_bo = umq,
		.log_buf_bo = logb,
		.max_opc = 0,
		.num_tiles = num_tiles,
		.mem_size = mem_size,
		.umq_doorbell = 0,
		.handle = 0,
		.syncobj_handle = 0,
	};
	unsigned long req =
		DRM_IOCTL_AMDXDNA(DRM_AMDXDNA_CREATE_HWCTX, struct amdxdna_drm_create_hwctx);
	if (xioctl(fd, req, &hw, "CREATE_HWCTX") < 0)
		goto out_close;
	printf("     hwctx handle=%u doorbell=%u\n", hw.handle, hw.umq_doorbell);

	if (create_bo(fd, AMDXDNA_BO_CMD, cmd_size, &cmd) < 0)
		goto out_destroy_hwctx;

	(void)map_and_fill_cmd_bo(fd, cmd, (size_t)cmd_size, fill_header);

	/* EXEC_CMD: single cmd BO handle as u64 (driver accepts scalar when count=1). */
	struct amdxdna_drm_exec_cmd ex = {
		.ext = 0,
		.ext_flags = 0,
		.hwctx = hw.handle,
		.type = AMDXDNA_CMD_SUBMIT_EXEC_BUF,
		.cmd_handles = (uint64_t)cmd,
		.args = 0,
		.cmd_count = 1,
		.arg_count = 0,
		.seq = 0,
	};
	req = DRM_IOCTL_AMDXDNA(DRM_AMDXDNA_EXEC_CMD, struct amdxdna_drm_exec_cmd);

	printf("\n>>> submitting EXEC_CMD (incomplete/scaffold CMD BO) …\n");
	fflush(stdout);

	if (ioctl(fd, req, &ex) < 0) {
		fprintf(stderr, "EXEC_CMD returned errno=%d (%s) seq=%" PRIu64 "\n", errno,
			strerror(errno), (uint64_t)ex.seq);
		fprintf(stderr,
			"If this is -EPERM, lab module may have exec_cmd_allow=0.\n"
			"If errno without oops: L2 guards or other validation may have worked.\n"
			"Check: journalctl -k -b | rg -i 'fill_slot|Oops|amdxdna'\n");
	} else {
		printf("EXEC_CMD ioctl returned success seq=%" PRIu64
		       " (unexpected for scaffold; check firmware path)\n",
		       (uint64_t)ex.seq);
	}

	printf("\nDone userspace path. If the kernel oopsed, this process may hang on "
	       "close/destroy.\n");

out_destroy_hwctx : {
	struct amdxdna_drm_destroy_hwctx dh = { .handle = hw.handle, .pad = 0 };
	req = DRM_IOCTL_AMDXDNA(DRM_AMDXDNA_DESTROY_HWCTX, struct amdxdna_drm_destroy_hwctx);
	(void)ioctl(fd, req, &dh);
}
out_close:
	close(fd);
	return 0;
}
