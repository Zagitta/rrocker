use anyhow::{Context, Result};
#[cfg(target_family = "unix")]
use nix::unistd::{Gid, Uid};
use std::{fs::OpenOptions, io::Write};

#[cfg(target_family = "unix")]
pub(crate) fn write_uid_map(inside: Uid, outside: Uid, len: u32) -> Result<()> {
    OpenOptions::new()
        .write(true)
        .open("/proc/self/uid_map")
        .context("Failed to open /proc/self/uid_map, is /proc mounted?")?
        .write_all(format!("{} {} {}", inside, outside, len).as_bytes())
        .context("Failed to write uid_map")
}

#[cfg(target_family = "unix")]
pub(crate) fn write_gid_map(inside: Gid, outside: Gid, len: u32) -> Result<()> {
    //Must be done due to the following restriction:
    //https://man7.org/linux/man-pages/man7/user_namespaces.7.html
    /* In the case of gid_map, use of the setgroups(2) system
       call must first be denied by writing "deny" to the
       /proc/[pid]/setgroups file (see below) before writing to
       gid_map.
    */
    OpenOptions::new()
        .write(true)
        .open("/proc/self/setgroups")
        .context("Failed to open /proc/self/setgroups, is /proc mounted?")?
        .write_all(b"deny")
        .context("Failed to write to /proc/self/setgroups")?;

    OpenOptions::new()
        .write(true)
        .open("/proc/self/gid_map")
        .context("Failed to open /proc/self/gid_map, is /proc mounted?")?
        .write_all(format!("{} {} {}", inside, outside, len).as_bytes())
        .context("Failed to write gid_map")
}

// Dummy funcs so stuff compiles on windows at least
#[cfg(target_family = "windows")]
pub(crate) fn write_uid_map(inside: u32, outside: u32, len: u32) -> Result<()> {
    unimplemented!()
}
#[cfg(target_family = "windows")]
pub(crate) fn write_gid_map(inside: u32, outside: u32, len: u32) -> Result<()> {
    unimplemented!()
}
