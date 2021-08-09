use anyhow::{Context, Result};
use nix::{
    mount::{self, MntFlags, MsFlags},
    unistd,
};
use std::path::{Path, PathBuf};

pub(crate) fn remount_private() -> Result<()> {
    nix::mount::mount(
        Option::<&str>::None,
        "/",
        Option::<&str>::None,
        MsFlags::MS_REC | MsFlags::MS_PRIVATE,
        Option::<&str>::None,
    )
    .context("Failed to remount privately")
}

pub(crate) fn mount_proc() -> Result<()> {
    const NAME: Option<&'static str> = Some("proc");
    let path = PathBuf::from("/proc");

    if !path.exists() {
        std::fs::create_dir(&path).context("Failed to create /proc dir")?
    }

    mount::mount(NAME, &path, NAME, MsFlags::empty(), Option::<&str>::None)
        .context("Failed to mount /proc")
}

pub(crate) fn pivot_root(root: &Path) -> Result<()> {
    const OLD_ROOT_NAME: &str = ".old_root";
    let old_root = root.join(OLD_ROOT_NAME);
    if !old_root.exists() {
        std::fs::create_dir(&old_root).context(format!("Failed to create '{:?}' dir", old_root))?;
    }

    mount::mount(
        Some(root),
        root,
        Option::<&str>::None,
        MsFlags::MS_REC | MsFlags::MS_BIND | MsFlags::MS_PRIVATE,
        Option::<&str>::None,
    )
    .context("Failed to (re)mount pivot root on top of itself as a bind mount")?;

    unistd::pivot_root(root, &old_root).context("Failed to pivot_root")?;

    unistd::chdir("/").context("Failed to change dir to /")?;

    //after pivoting the old root folder has moved
    let inner_old_root = Path::new("/").join(OLD_ROOT_NAME);

    mount::umount2(&inner_old_root, MntFlags::MNT_DETACH)
        .context(format!("Failed to unmount '{:?}'", old_root))?;

    Ok(())
}

#[allow(dead_code)]
pub(crate) fn unmount_all() -> Result<()> {
    mount::umount2("/", MntFlags::MNT_DETACH).context("Failed to unmount /")
}

pub(crate) fn mount_sysfs() -> Result<()> {
    let p = Path::new("/sys");

    if !p.exists() {
        std::fs::create_dir_all(&p).context("Failed to create '/sys' path")?;
    }

    mount::mount(
        Option::<&str>::None,
        p,
        Some("sysfs"),
        MsFlags::empty(),
        Option::<&str>::None,
    )
    .context("Failed to mount sysfs")?;

    Ok(())
}

#[allow(dead_code)]
pub(crate) fn mount_cgroups() -> Result<()> {
    mount::mount(
        Option::<&str>::None,
        "/sys/fs/cgroup",
        Some("cgroup2"),
        MsFlags::empty(),
        Option::<&str>::None,
    )
    .context("Failed to mount cgroup2")?;

    Ok(())
}
