/* LD_PRELOAD ioctl dumper. Does not open /dev/accel.
   Does not set XDNA2_ALLOW_EXEC_CMD. Does not include amdxdna_accel.h.
   Init uses the syscall instruction so a hooked open cannot deadlock dlsym. */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/syscall.h>
#include <unistd.h>

#define MAP_SLOTS 128
#define DUMP_CAP 65536u
#define FOLLOW_CAP 16384u

struct maprec {
    uintptr_t addr;
    size_t len;
    int fd;
    unsigned long long offset;
};

static int (*real_ioctl)(int, unsigned long, void *);
static long (*real_syscall)(long, long, long, long, long, long, long);
static int (*real_open)(const char *, int, int);
static int (*real_openat)(int, const char *, int, int);
static void *(*real_mmap)(void *, size_t, int, int, int, off_t);
static int (*real_munmap)(void *, size_t);
static int (*real_dup)(int);
static int (*real_dup2)(int, int);
static int (*real_dup3)(int, int, int);

static FILE *dump_fp;
static int ready;
static int ensuring;
static __thread int in_ensure;
static __thread int in_hook;
static int lock_word;
static int accel_fds[256];
static struct maprec maps[MAP_SLOTS];
static int map_count;

static long raw_syscall(long n, long a1, long a2, long a3, long a4, long a5, long a6) {
    long ret;
    register long r10 __asm__("r10") = a4;
    register long r8 __asm__("r8") = a5;
    register long r9 __asm__("r9") = a6;
    __asm__ volatile("syscall"
                     : "=a"(ret)
                     : "a"(n), "D"(a1), "S"(a2), "d"(a3), "r"(r10), "r"(r8), "r"(r9)
                     : "rcx", "r11", "memory");
    return ret;
}

static void dolock(void) {
    while (__sync_lock_test_and_set(&lock_word, 1)) {
    }
}

static void unlock(void) { __sync_lock_release(&lock_word); }

static int is_accel_fd(int fd) {
    if (fd < 0 || fd >= (int)(sizeof accel_fds)) {
        return 0;
    }
    return accel_fds[fd];
}

static void mark_fd(int fd, int on) {
    if (fd < 0 || fd >= (int)(sizeof accel_fds)) {
        return;
    }
    accel_fds[fd] = on;
}

static int path_is_accel(const char *path) {
    return path != NULL && (strstr(path, "accel") != NULL || strstr(path, "amdxdna") != NULL);
}

static void ensure(void) {
    if (ready || in_ensure) {
        return;
    }
    in_ensure = 1;
    if (!__sync_bool_compare_and_swap(&ensuring, 0, 1)) {
        in_ensure = 0;
        while (!ready) {
        }
        return;
    }
    real_ioctl = (int (*)(int, unsigned long, void *))dlsym(RTLD_NEXT, "ioctl");
    real_syscall = (long (*)(long, long, long, long, long, long, long))dlsym(RTLD_NEXT, "syscall");
    real_open = (int (*)(const char *, int, int))dlsym(RTLD_NEXT, "open");
    real_openat = (int (*)(int, const char *, int, int))dlsym(RTLD_NEXT, "openat");
    real_mmap = (void *(*)(void *, size_t, int, int, int, off_t))dlsym(RTLD_NEXT, "mmap");
    real_munmap = (int (*)(void *, size_t))dlsym(RTLD_NEXT, "munmap");
    real_dup = (int (*)(int))dlsym(RTLD_NEXT, "dup");
    real_dup2 = (int (*)(int, int))dlsym(RTLD_NEXT, "dup2");
    real_dup3 = (int (*)(int, int, int))dlsym(RTLD_NEXT, "dup3");
    const char *path = getenv("DRAGON_IOCTL_DUMP");
    if (path == NULL || path[0] == '\0') {
        path = "/tmp/grok-goal-3ac8ef958e6f/implementer/ioctl-dump.txt";
    }
    int fd = (int)raw_syscall(SYS_openat, AT_FDCWD, (long)path, O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0644, 0, 0);
    if (fd >= 0) {
        dump_fp = fdopen(fd, "w");
        if (dump_fp != NULL) {
            fprintf(dump_fp, "preload ioctl_arg_dump_preload\n");
            fflush(dump_fp);
        }
    }
    ready = 1;
    in_ensure = 0;
}

static size_t readable_prefix(const void *p, size_t n) {
    if (p == NULL || n == 0) {
        return 0;
    }
    const unsigned char *b = p;
    size_t ok = 0;
    while (ok < n) {
        uintptr_t addr = (uintptr_t)(b + ok);
        size_t chunk = 4096u - (size_t)(addr & 4095u);
        if (chunk > n - ok) {
            chunk = n - ok;
        }
        unsigned char vec = 0;
        if (mincore((void *)(addr & ~(uintptr_t)4095u), 4096, &vec) != 0) {
            break;
        }
        ok += chunk;
    }
    return ok;
}

static int looks_like_user_ptr(unsigned long long value) {
    return value >= 0x10000ull && value <= 0x00007fffffffffffULL;
}

static void hex_dump(const char *label, const unsigned char *bytes, size_t n) {
    if (dump_fp == NULL) {
        return;
    }
    fprintf(dump_fp, "%s len=%zu\n", label, n);
    for (size_t i = 0; i < n; i += 16) {
        fprintf(dump_fp, "%04zx:", i);
        for (size_t j = 0; j < 16 && i + j < n; j++) {
            fprintf(dump_fp, " %02x", bytes[i + j]);
        }
        fprintf(dump_fp, "\n");
    }
}

static void add_map(uintptr_t addr, size_t len, int fd, unsigned long long offset) {
    if (len == 0) {
        return;
    }
    int slot = map_count < MAP_SLOTS ? map_count++ : MAP_SLOTS - 1;
    maps[slot].addr = addr;
    maps[slot].len = len;
    maps[slot].fd = fd;
    maps[slot].offset = offset;
}

static void drop_map(uintptr_t addr) {
    for (int i = 0; i < map_count; i++) {
        if (maps[i].addr == addr) {
            maps[i] = maps[map_count - 1];
            map_count--;
            return;
        }
    }
}

static struct maprec *map_containing(uintptr_t addr) {
    for (int i = 0; i < map_count; i++) {
        if (addr >= maps[i].addr && addr < maps[i].addr + maps[i].len) {
            return &maps[i];
        }
    }
    return NULL;
}

static void follow_region(const unsigned char *bytes, size_t n, int depth, uintptr_t seen[], int *seen_n) {
    if (depth > 2 || bytes == NULL) {
        return;
    }
    size_t limit = n < FOLLOW_CAP ? n : FOLLOW_CAP;
    for (size_t off = 0; off + 8 <= limit; off += 8) {
        unsigned long long value = 0;
        memcpy(&value, bytes + off, 8);
        if (!looks_like_user_ptr(value)) {
            continue;
        }
        int already = 0;
        for (int s = 0; s < *seen_n; s++) {
            if (seen[s] == (uintptr_t)value) {
                already = 1;
                break;
            }
        }
        if (already || *seen_n >= 64) {
            continue;
        }
        seen[(*seen_n)++] = (uintptr_t)value;
        struct maprec *inside = map_containing((uintptr_t)value);
        size_t want = inside != NULL ? DUMP_CAP : FOLLOW_CAP;
        if (inside != NULL) {
            size_t room = (inside->addr + inside->len) - (uintptr_t)value;
            if (want > room) {
                want = room;
            }
        }
        size_t got = readable_prefix((void *)(uintptr_t)value, want);
        if (got == 0) {
            continue;
        }
        char label[160];
        snprintf(label, sizeof label, "follow depth=%d from_off=0x%zx ptr=0x%llx", depth, off, value);
        hex_dump(label, (const unsigned char *)(uintptr_t)value, got);
        follow_region((const unsigned char *)(uintptr_t)value, got, depth + 1, seen, seen_n);
    }
}

static void dump_small_maps(void) {
    for (int i = 0; i < map_count; i++) {
        if (maps[i].len == 0 || maps[i].len > DUMP_CAP) {
            continue;
        }
        size_t got = readable_prefix((void *)maps[i].addr, maps[i].len);
        if (got == 0) {
            continue;
        }
        char label[192];
        snprintf(label, sizeof label, "mmap fd=%d offset=0x%llx base=0x%llx", maps[i].fd, maps[i].offset,
                 (unsigned long long)maps[i].addr);
        hex_dump(label, (const unsigned char *)maps[i].addr, got);
    }
}

static void dump_ioctl(const char *phase, int fd, unsigned long req, void *arg, long ret) {
    if (dump_fp == NULL) {
        return;
    }
    unsigned size = (unsigned)((req >> 16) & 0x3fffu);
    unsigned type = (unsigned)((req >> 8) & 0xffu);
    if (!is_accel_fd(fd) && type != (unsigned)'d') {
        return;
    }
    dolock();
    fprintf(dump_fp, "ioctl phase=%s fd=%d accel=%d request=0x%08lx type=%u nr=%lu size=%u ret=%ld\n", phase, fd,
            is_accel_fd(fd), req, type, req & 0xfful, size, ret);
    if (size > 0 && arg != NULL) {
        size_t got = readable_prefix(arg, size);
        char label[64];
        snprintf(label, sizeof label, "arg phase=%s", phase);
        if (got > 0) {
            hex_dump(label, (const unsigned char *)arg, got);
            if (strcmp(phase, "before") == 0) {
                uintptr_t seen[64];
                int seen_n = 0;
                follow_region((const unsigned char *)arg, got, 1, seen, &seen_n);
                if (size >= 0x30u) {
                    dump_small_maps();
                }
            }
        } else {
            fprintf(dump_fp, "arg phase=%s unreadable size=%u\n", phase, size);
        }
    }
    fflush(dump_fp);
    unlock();
}

int ioctl(int fd, unsigned long req, ...) {
    va_list ap;
    va_start(ap, req);
    void *arg = va_arg(ap, void *);
    va_end(ap);
    if (!ready) {
        if (in_ensure) {
            return (int)raw_syscall(SYS_ioctl, fd, (long)req, (long)arg, 0, 0, 0);
        }
        ensure();
    }
    if (real_ioctl == NULL) {
        return (int)raw_syscall(SYS_ioctl, fd, (long)req, (long)arg, 0, 0, 0);
    }
    if (!in_hook) {
        in_hook = 1;
        dump_ioctl("before", fd, req, arg, 0);
        in_hook = 0;
    }
    int ret = real_ioctl(fd, req, arg);
    if (!in_hook) {
        in_hook = 1;
        dump_ioctl("after", fd, req, arg, ret);
        in_hook = 0;
    }
    return ret;
}

long syscall(long n, ...) {
    va_list ap;
    va_start(ap, n);
    long a1 = va_arg(ap, long);
    long a2 = va_arg(ap, long);
    long a3 = va_arg(ap, long);
    long a4 = va_arg(ap, long);
    long a5 = va_arg(ap, long);
    long a6 = va_arg(ap, long);
    va_end(ap);
    if (!ready) {
        if (in_ensure) {
            return raw_syscall(n, a1, a2, a3, a4, a5, a6);
        }
        ensure();
    }
    if (real_syscall == NULL) {
        return raw_syscall(n, a1, a2, a3, a4, a5, a6);
    }
    if (!in_hook && n == SYS_ioctl) {
        in_hook = 1;
        dump_ioctl("before", (int)a1, (unsigned long)a2, (void *)a3, 0);
        in_hook = 0;
    }
    long ret = real_syscall(n, a1, a2, a3, a4, a5, a6);
    if (!in_hook && n == SYS_ioctl) {
        in_hook = 1;
        dump_ioctl("after", (int)a1, (unsigned long)a2, (void *)a3, ret);
        in_hook = 0;
    }
    return ret;
}

int open(const char *path, int flags, ...) {
    int mode = 0;
    if ((flags & O_CREAT) != 0) {
        va_list ap;
        va_start(ap, flags);
        mode = va_arg(ap, int);
        va_end(ap);
    }
    if (!ready) {
        if (in_ensure) {
            return (int)raw_syscall(SYS_openat, AT_FDCWD, (long)path, flags, mode, 0, 0);
        }
        ensure();
    }
    if (real_open == NULL) {
        return (int)raw_syscall(SYS_openat, AT_FDCWD, (long)path, flags, mode, 0, 0);
    }
    int fd = real_open(path, flags, mode);
    if (!in_hook && fd >= 0) {
        mark_fd(fd, path_is_accel(path));
    }
    return fd;
}

int openat(int dirfd, const char *path, int flags, ...) {
    int mode = 0;
    if ((flags & O_CREAT) != 0) {
        va_list ap;
        va_start(ap, flags);
        mode = va_arg(ap, int);
        va_end(ap);
    }
    if (!ready) {
        if (in_ensure) {
            return (int)raw_syscall(SYS_openat, dirfd, (long)path, flags, mode, 0, 0);
        }
        ensure();
    }
    if (real_openat == NULL) {
        return (int)raw_syscall(SYS_openat, dirfd, (long)path, flags, mode, 0, 0);
    }
    int fd = real_openat(dirfd, path, flags, mode);
    if (!in_hook && fd >= 0 && path_is_accel(path)) {
        mark_fd(fd, 1);
    }
    return fd;
}

void *mmap(void *addr, size_t length, int prot, int flags, int fd, off_t offset) {
    if (!ready) {
        if (in_ensure) {
            errno = ENOSYS;
            return MAP_FAILED;
        }
        ensure();
    }
    if (real_mmap == NULL) {
        errno = ENOSYS;
        return MAP_FAILED;
    }
    void *p = real_mmap(addr, length, prot, flags, fd, offset);
    if (!in_hook && p != MAP_FAILED && is_accel_fd(fd)) {
        dolock();
        add_map((uintptr_t)p, length, fd, (unsigned long long)offset);
        if (dump_fp != NULL) {
            fprintf(dump_fp, "mmap fd=%d offset=0x%llx base=0x%llx len=%zu prot=%d\n", fd,
                    (unsigned long long)offset, (unsigned long long)(uintptr_t)p, length, prot);
            fflush(dump_fp);
        }
        unlock();
    }
    return p;
}

int munmap(void *addr, size_t length) {
    if (!ready) {
        if (!in_ensure) {
            ensure();
        }
    }
    if (real_munmap == NULL) {
        errno = ENOSYS;
        return -1;
    }
    if (!in_hook && addr != NULL) {
        dolock();
        drop_map((uintptr_t)addr);
        unlock();
    }
    return real_munmap(addr, length);
}

int dup(int oldfd) {
    if (!ready && !in_ensure) {
        ensure();
    }
    if (real_dup == NULL) {
        errno = ENOSYS;
        return -1;
    }
    int fd = real_dup(oldfd);
    if (!in_hook && fd >= 0 && is_accel_fd(oldfd)) {
        mark_fd(fd, 1);
    }
    return fd;
}

int dup2(int oldfd, int newfd) {
    if (!ready && !in_ensure) {
        ensure();
    }
    if (real_dup2 == NULL) {
        errno = ENOSYS;
        return -1;
    }
    int fd = real_dup2(oldfd, newfd);
    if (!in_hook && fd >= 0) {
        mark_fd(fd, is_accel_fd(oldfd));
    }
    return fd;
}

int dup3(int oldfd, int newfd, int flags) {
    if (!ready && !in_ensure) {
        ensure();
    }
    if (real_dup3 == NULL) {
        errno = ENOSYS;
        return -1;
    }
    int fd = real_dup3(oldfd, newfd, flags);
    if (!in_hook && fd >= 0) {
        mark_fd(fd, is_accel_fd(oldfd));
    }
    return fd;
}

__attribute__((constructor)) static void preload_init(void) { ensure(); }
