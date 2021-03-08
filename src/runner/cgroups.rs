use crate::runner::service::{run_request, RunRequest};

use controlgroup::v1::{Builder, UnifiedRepr};

use anyhow::{Context, Result};
use controlgroup::v1::Cgroup;
use controlgroup::Device;
use std::path::PathBuf;
use tokio::process::Command;
use uuid::Uuid;

pub fn create_cgroups(request: &RunRequest, id: &Uuid) -> Result<UnifiedRepr> {
    let mut builder = Builder::new(PathBuf::from(id.to_string()));

    if let Some(run_request::Memory::MaxMemory(max)) = request.memory {
        builder = builder.memory().limit_in_bytes(max).done();
    }

    if let Some(run_request::Cpu::MaxCpu(max)) = request.cpu {
        builder = builder.cpu().shares(max).done();
    }

    if let Some(run_request::Disk::MaxDisk(max)) = request.disk {
        // The blkio.weight and similar are often disabled on some
        // of the latest distros (Ubuntu 20.04 being one example)
        // Let's iterate through found block devices and set read
        // and write bps for the sake of disk constraining
        // requirement of this proof-of-concept

        let mut enumerator = udev::Enumerator::new().unwrap();
        enumerator.match_subsystem("block").unwrap();

        let devices = enumerator
            .scan_devices()?
            .filter_map(|device| {
                if let Some(devnum) = device.devnum() {
                    let major = (devnum & 0xFF00) >> 8;
                    let minor = devnum & 0xFFFF00FF;

                    device.devtype().and_then(|devtype| {
                        devtype.to_str().and_then(|typ| {
                            if typ == "disk" {
                                Some((Device::from([major as u16, minor as u16]), max))
                            } else {
                                None
                            }
                        })
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<(Device, u64)>>();

        builder = builder
            .blkio()
            .read_bps_device(devices.iter().copied())
            .write_bps_device(devices.iter().copied())
            .done();
    }

    builder
        .build()
        .context("Couldn't create a Linux control group for the new process")
}

pub fn apply_cgroup_pre_exec<C: Cgroup>(cmd: &mut Command, cgroup: &C) {
    let path = cgroup.path().join("cgroup.procs");

    unsafe {
        cmd.pre_exec(move || std::fs::write(&path, std::process::id().to_string()));
    }
}
