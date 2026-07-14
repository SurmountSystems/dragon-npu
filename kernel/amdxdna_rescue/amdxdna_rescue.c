// SPDX-License-Identifier: GPL-2.0
/*
 * amdxdna_rescue — recover a wedged amdxdna path without rebooting the machine.
 *
 * Context: bad EXEC_CMD/cmdlist work left the driver oops'd; subsequent open()
 * calls hang in amdxdna_drm_open (TASK_UNINTERRUPTIBLE). Module refcnt stays
 * high; rmmod fails. This helper is loaded with insmod and talks only via dmesg.
 *
 * Actions (module param `action=`):
 *   diagnose   — dump stuck tasks (default)
 *   reset_npu  — pci_reset_function / try_reset on 1022:17f0
 *   reset_bus  — secondary bus reset via parent bridge (also hits dummy 1116)
 *   wake       — wake_up_process on amdxdna-related sleepers (after reset)
 *   full       — diagnose + reset_npu + reset_bus + wake
 *
 * After a successful reset, userspace should:
 *   echo 1 > /sys/bus/pci/devices/0000:c6:00.1/remove
 *   echo 1 > /sys/bus/pci/rescan
 *   modprobe -r amdxdna; modprobe amdxdna
 *
 * Danger: PCI reset is real hardware reset of the NPU (and bus reset also the
 * dummy function on the same bus). Prefer over force-rmmod; still not free.
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/pci.h>
#include <linux/sched.h>
#include <linux/sched/signal.h>
#include <linux/string.h>

#define DRV "amdxdna_rescue"
#define AMD_NPU_VENDOR 0x1022
#define AMD_NPU_DEVICE 0x17f0

static char *action = "diagnose";
module_param(action, charp, 0444);
MODULE_PARM_DESC(action, "diagnose|reset_npu|reset_bus|wake|full");

static bool task_is_deep_sleep(struct task_struct *p)
{
	unsigned int st = READ_ONCE(p->__state);

	/* Uninterruptible and killable-uninterruptible sleeps */
	return (st & TASK_UNINTERRUPTIBLE) != 0;
}

/*
 * get_wchan is not EXPORT_SYMBOL on many kernels — match by task->comm and
 * uninterruptible state instead. Userspace diagnose script has wchan via /proc.
 */
static int comm_looks_npu_stuck(const char *comm)
{
	if (!comm)
		return 0;
	return strstr(comm, "npu_direct") || strstr(comm, "dragon_npu") ||
	       strstr(comm, "real_xdna") || strstr(comm, "xdna2") ||
	       strstr(comm, "amdxdna") || !strcmp(comm, "cat") ||
	       !strcmp(comm, "timeout");
}

static void rescue_dump_stuck_tasks(void)
{
	struct task_struct *g, *p;
	int n = 0, n_all_d = 0;

	pr_warn(DRV ": === stuck task scan (D-state + npu-ish comm) ===\n");
	rcu_read_lock();
	for_each_process_thread(g, p) {
		if (!task_is_deep_sleep(p))
			continue;
		n_all_d++;
		if (!comm_looks_npu_stuck(p->comm))
			continue;
		n++;
		pr_warn(DRV ": pid=%d tgid=%d comm=%s state=0x%x\n", p->pid,
			p->tgid, p->comm, READ_ONCE(p->__state));
	}
	rcu_read_unlock();
	pr_warn(DRV ": npu-ish D tasks=%d (all D threads seen=%d)\n", n, n_all_d);
}

static int rescue_reset_function(struct pci_dev *pdev)
{
	int ret;

	pr_warn(DRV ": pci_reset_function(%s)\n", pci_name(pdev));
	ret = pci_reset_function(pdev);
	if (!ret) {
		pr_warn(DRV ": function reset OK\n");
		return 0;
	}
	pr_warn(DRV ": pci_reset_function -> %d, trying pci_try_reset_function\n",
		ret);
	ret = pci_try_reset_function(pdev);
	pr_warn(DRV ": pci_try_reset_function -> %d\n", ret);
	return ret;
}

static int rescue_reset_bus(struct pci_dev *pdev)
{
	struct pci_dev *bridge;
	int ret;

	if (!pdev->bus || !pdev->bus->self) {
		pr_err(DRV ": no parent bridge for bus reset\n");
		return -ENODEV;
	}
	bridge = pdev->bus->self;
	pr_warn(DRV ": secondary bus reset via %s (child NPU %s)\n",
		pci_name(bridge), pci_name(pdev));
	/*
	 * On this platform (Krackan): parent 00:08.2 has sysfs reset; NPU
	 * 17f0 often has no per-function reset file. Bus reset is the lever.
	 */
	ret = pci_bridge_secondary_bus_reset(bridge);
	pr_warn(DRV ": pci_bridge_secondary_bus_reset -> %d\n", ret);
	return ret;
}

static void rescue_wake(void)
{
	struct task_struct *g, *p;
	int n = 0;

	pr_warn(DRV ": wake_up_process on deep-sleep npu-ish tasks\n");
	rcu_read_lock();
	for_each_process_thread(g, p) {
		if (!task_is_deep_sleep(p))
			continue;
		if (!comm_looks_npu_stuck(p->comm))
			continue;
		wake_up_process(p);
		n++;
	}
	rcu_read_unlock();
	pr_warn(DRV ": wake issued to %d task(s)\n", n);
}

static int __init amdxdna_rescue_init(void)
{
	struct pci_dev *pdev;
	int ret = 0;
	bool want_diag = true;
	bool want_npu = false;
	bool want_bus = false;
	bool want_wake = false;

	pr_warn(DRV ": init action=%s\n", action);

	if (!strcmp(action, "diagnose")) {
		/* defaults */
	} else if (!strcmp(action, "reset_npu")) {
		want_npu = true;
	} else if (!strcmp(action, "reset_bus")) {
		want_bus = true;
	} else if (!strcmp(action, "wake")) {
		want_wake = true;
	} else if (!strcmp(action, "full")) {
		want_npu = true;
		want_bus = true;
		want_wake = true;
	} else {
		pr_err(DRV ": bad action '%s'\n", action);
		return -EINVAL;
	}

	if (want_diag)
		rescue_dump_stuck_tasks();

	pdev = pci_get_device(AMD_NPU_VENDOR, AMD_NPU_DEVICE, NULL);
	if (!pdev) {
		pr_err(DRV ": NPU %04x:%04x not found on PCI\n", AMD_NPU_VENDOR,
		       AMD_NPU_DEVICE);
		if (want_npu || want_bus)
			return -ENODEV;
		return 0;
	}

	pr_warn(DRV ": NPU %s driver=%s\n", pci_name(pdev),
		pdev->driver ? pdev->driver->name : "(none)");

	if (want_npu) {
		ret = rescue_reset_function(pdev);
		/* Many NPUs return -ENOTTY; bus reset is the real tool. */
		if (ret)
			pr_warn(DRV ": function reset unsupported/failed; try reset_bus\n");
	}

	if (want_bus) {
		int r = rescue_reset_bus(pdev);

		if (r)
			ret = r;
		else
			ret = 0;
	}

	if (want_wake)
		rescue_wake();

	/* Re-scan after reset for evidence */
	if (want_npu || want_bus || want_wake)
		rescue_dump_stuck_tasks();

	pci_dev_put(pdev);

	/* diagnose/wake always "succeed" insmod so dmesg is collected */
	if (!want_npu && !want_bus)
		return 0;
	return ret;
}

static void __exit amdxdna_rescue_exit(void)
{
	pr_warn(DRV ": exit\n");
}

module_init(amdxdna_rescue_init);
module_exit(amdxdna_rescue_exit);

MODULE_LICENSE("GPL");
MODULE_AUTHOR("dragon-npu");
MODULE_DESCRIPTION("Engineering rescue for wedged amdxdna (no full reboot)");
MODULE_VERSION("0.2");
