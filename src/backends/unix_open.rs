//! Pure Unix discretionary check for a read-write open.
//!
//! No device node is opened here. Callers pass mode, owner, and caller ids.

/// Whether Unix mode bits allow opening a node read-write.
///
/// `mode` may be a raw `st_mode`. File-type bits are ignored. The owner class
/// is used when `caller_euid` equals `owner_uid`. Otherwise the group class
/// is used when `caller_egid` or one supplementary group equals `owner_gid`.
/// Otherwise the other class is used. Read-write needs both the read bit and
/// the write bit of that one class. Effective uid 0 is allowed. This is not
/// an ACL check and it does not open a device.
#[must_use]
pub fn unix_open_read_write_allowed(
    mode: u32,
    owner_uid: u32,
    owner_gid: u32,
    caller_euid: u32,
    caller_egid: u32,
    supplementary_groups: &[u32],
) -> bool {
    // Root bypasses the mode class. Checked before owner so mode 000 still allows euid 0.
    if caller_euid == 0 {
        return true;
    }
    // File-type bits sit above the permission triplet. Shifting then masking drops them.
    let class = (if caller_euid == owner_uid {
        mode >> 6
    } else if caller_egid == owner_gid || supplementary_groups.contains(&owner_gid) {
        mode >> 3
    } else {
        mode
    }) & 0o7;
    let read = 0o4;
    let write = 0o2;
    class & read == read && class & write == write
}

#[cfg(test)]
mod tests {
    use super::unix_open_read_write_allowed;

    #[test]
    fn unix_open_read_write_allowed_by_mode_owner_and_group() {
        assert!(
            unix_open_read_write_allowed(0o666, 0, 0, 1000, 1000, &[]),
            "mode 0666 must allow a non-root caller"
        );
        assert!(
            unix_open_read_write_allowed(0o20_0666, 0, 0, 1000, 1000, &[]),
            "character-device mode 0666 must allow a non-root caller"
        );
        assert!(
            unix_open_read_write_allowed(0o660, 0, 44, 1000, 1000, &[44]),
            "mode 0660 must allow a caller in the device group"
        );
        assert!(
            !unix_open_read_write_allowed(0o660, 0, 44, 1000, 1000, &[]),
            "mode 0660 must refuse a caller who is not in the device group"
        );
        assert!(
            !unix_open_read_write_allowed(0o600, 0, 0, 1000, 1000, &[]),
            "mode 0600 must refuse a non-owner"
        );
    }
}
