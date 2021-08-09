use std::path::Path;

use crate::{
    clone_context::{CloneContext, ResultReader},
    fs, user,
};
use anyhow::{Context, Result};
use nix::unistd::{Gid, Pid, Uid};
use serde::{de::DeserializeOwned, Serialize};
pub struct IsolatedProcess<'a, T: Serialize + DeserializeOwned + Send> {
    ctx: CloneContext<'a, T>,
}

const ROOT_UID: Uid = Uid::from_raw(0);
const ROOT_GID: Gid = Gid::from_raw(0);

impl<'a, T: Serialize + DeserializeOwned + Send> IsolatedProcess<'a, T> {
    pub fn new<F: 'a + FnMut() -> Result<T>>(mut func: F) -> Result<Self> {
        let gid = Gid::current();
        let uid = Uid::current();
        Ok(Self {
            ctx: CloneContext::new(move || -> Result<T> {
                fs::remount_private().context("Failed to remount privately")?;
                fs::pivot_root(&Path::new("/var/rrocker-root/")).context("Failed to pivot root")?;
                fs::mount_proc().context("Failed to mount proc")?;
                fs::mount_sysfs().context("Failed to mount sysfs")?;
                //fs::mount_cgroups().context("Failed to mount cgroup")?;
                user::write_gid_map(ROOT_GID, gid, 1).context("Failed to write gid map")?;
                user::write_uid_map(ROOT_UID, uid, 1).context("Failed to write uid map")?;

                func()
            })
            .context("Failed to create ctx of IsolatedProcess")?,
        })
    }

    pub fn execute(self) -> Result<(Pid, ResultReader<T>)> {
        self.ctx.execute()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn is_pid_isolated() {
        use sysinfo::{System, SystemExt};

        let cc = IsolatedProcess::new(|| -> Result<Vec<i32>> {
            let mut sys = System::new();
            sys.refresh_processes();

            Ok(sys.processes().iter().map(|(k, _p)| *k).collect())
        })
        .unwrap();

        let (_, mut rr) = cc.execute().unwrap();

        match rr.get_result() {
            Ok(pids) => assert_eq!(pids, vec![1i32]), //ensure we can only see ourselves the init pid
            Err(e) => assert!(false, "{:?}", e),
        }
    }

    #[test]
    fn is_net_isolated() {
        use sysinfo::{System, SystemExt};

        let cc = IsolatedProcess::new(|| -> Result<Vec<String>> {
            let mut sys = System::new();
            sys.refresh_networks_list();

            Ok(sys
                .networks()
                .into_iter()
                .map(|(name, _net)| name.clone())
                .collect())
        })
        .unwrap();

        let (_, mut rr) = cc.execute().unwrap();

        match rr.get_result() {
            Ok(network_names) => assert!(network_names.is_empty()), //ensure no network access
            Err(e) => assert!(false, "{:?}", e),
        }
    }

    #[test]
    fn is_disk_isolated() {
        use sysinfo::{DiskExt, System, SystemExt};

        let cc = IsolatedProcess::new(|| -> Result<Vec<String>> {
            let mut sys = System::new();
            sys.refresh_disks();

            Ok(sys
                .disks()
                .into_iter()
                .flat_map(|d| d.name().to_owned().into_string())
                .collect())
        })
        .unwrap();

        let (_, mut rr) = cc.execute().unwrap();

        match rr.get_result() {
            Ok(disk_names) => assert!(disk_names.is_empty()), //ensure no network access
            Err(e) => assert!(false, "{:?}", e),
        }
    }
}
