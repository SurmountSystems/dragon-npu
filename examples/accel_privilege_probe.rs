//! Stat `/dev/accel/accel0`, apply the pure Unix read-write check, and open once.
//!
//! No ioctl. No CREATE_BO. No CREATE_HWCTX. No EXEC_CMD.

use std::fs::OpenOptions;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::process::{Command, ExitCode};

use earth_dragon_npu::backends::unix_open_read_write_allowed;

const DEVICE: &str = "/dev/accel/accel0";
const EXEC_ALLOW: &str = "/sys/module/amdxdna/parameters/exec_cmd_allow";

fn main() -> ExitCode {
    let stat = std::fs::metadata(DEVICE);
    match &stat {
        Ok(meta) => {
            println!(
                "stat path={DEVICE} mode={:06o} uid={} gid={}",
                meta.mode(),
                meta.uid(),
                meta.gid()
            );
        }
        Err(err) => {
            let code = err.raw_os_error().unwrap_or(0);
            println!(
                "stat path={DEVICE} result=no errno={code} name={}",
                errno_name(code)
            );
        }
    }
    print_command("id", &[]);
    print_command("id", &["-G"]);
    print_command("groups", &[]);
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    match field_rest(&status, "CapEff") {
        Some(value) => println!("CapEff={value}"),
        None => println!("CapEff=absent"),
    }
    let allow_value = print_exec_allow();
    if let Ok(meta) = &stat {
        let (euid, egid, groups) = caller_ids(&status);
        match (euid, egid) {
            (Some(euid), Some(egid)) => {
                let allowed = unix_open_read_write_allowed(
                    meta.mode(),
                    meta.uid(),
                    meta.gid(),
                    euid,
                    egid,
                    &groups,
                );
                println!(
                    "caller euid={euid} egid={egid} supplementary={groups:?} allowed={}",
                    if allowed { "yes" } else { "no" }
                );
            }
            _ => println!("allowed=not-computed caller ids missing from /proc/self/status"),
        }
    } else {
        println!("allowed=not-computed stat failed");
    }
    if allow_value.as_deref() == Some("0") {
        println!("exec_cmd_allow is 0. The kernel exec gate is separate and was not changed.");
    }
    match OpenOptions::new().read(true).write(true).open(DEVICE) {
        Ok(file) => {
            drop(file);
            println!("open path={DEVICE} result=yes closed=yes errno=none");
            println!("Unix permission is not required to open the node.");
            ExitCode::SUCCESS
        }
        Err(err) => {
            let code = err.raw_os_error().unwrap_or(0);
            println!(
                "open path={DEVICE} result=no errno={code} name={}",
                errno_name(code)
            );
            if code == 1 || code == 13 {
                println!("privilege is required");
            }
            ExitCode::from(1)
        }
    }
}

fn print_exec_allow() -> Option<String> {
    let path = Path::new(EXEC_ALLOW);
    if !path.exists() {
        println!("exec_cmd_allow path={EXEC_ALLOW} present=no");
        return None;
    }
    match std::fs::metadata(path) {
        Ok(meta) => println!(
            "exec_cmd_allow path={EXEC_ALLOW} present=yes mode={:06o}",
            meta.mode()
        ),
        Err(err) => {
            let code = err.raw_os_error().unwrap_or(0);
            println!(
                "exec_cmd_allow path={EXEC_ALLOW} present=yes mode_errno={code} name={}",
                errno_name(code)
            );
        }
    }
    match std::fs::read_to_string(path) {
        Ok(text) => {
            let value = text.trim().to_string();
            println!("exec_cmd_allow value={value}");
            Some(value)
        }
        Err(err) => {
            let code = err.raw_os_error().unwrap_or(0);
            println!(
                "exec_cmd_allow value_errno={code} name={}",
                errno_name(code)
            );
            None
        }
    }
}

fn print_command(program: &str, args: &[&str]) {
    match Command::new(program).args(args).output() {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            println!(
                "command program={program} args={args:?} status={} stdout={} stderr={}",
                out.status,
                stdout.trim(),
                stderr.trim()
            );
        }
        Err(err) => println!("command program={program} args={args:?} start_failed={err}"),
    }
}

fn caller_ids(status: &str) -> (Option<u32>, Option<u32>, Vec<u32>) {
    let euid = field_words(status, "Uid")
        .get(1)
        .and_then(|word| word.parse().ok());
    let egid = field_words(status, "Gid")
        .get(1)
        .and_then(|word| word.parse().ok());
    let groups = field_words(status, "Groups")
        .iter()
        .filter_map(|word| word.parse().ok())
        .collect();
    (euid, egid, groups)
}

fn field_rest<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    for line in text.lines() {
        let rest = line.strip_prefix(key)?.strip_prefix(':')?;
        return Some(rest.trim());
    }
    None
}

fn field_words(text: &str, key: &str) -> Vec<String> {
    field_rest(text, key)
        .map(|rest| rest.split_whitespace().map(str::to_string).collect())
        .unwrap_or_default()
}

fn errno_name(code: i32) -> &'static str {
    match code {
        1 => "EPERM",
        2 => "ENOENT",
        11 => "EAGAIN",
        13 => "EACCES",
        _ => "other",
    }
}
