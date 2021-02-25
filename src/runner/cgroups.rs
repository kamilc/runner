use crate::runner::service::{run_request, RunRequest};

use controlgroup::v1::{Builder, UnifiedRepr};

use anyhow::{Context, Result};
use std::convert::TryFrom;
use std::path::PathBuf;

pub fn create_cgroups(request: &RunRequest, id: &str) -> Result<UnifiedRepr> {
    let mut builder = Builder::new(PathBuf::from(id));

    if let Some(run_request::Memory::MaxMemory(max)) = request.memory {
        builder = builder.memory().limit_in_bytes(max).done();
    }

    if let Some(run_request::Cpu::MaxCpu(max)) = request.cpu {
        builder = builder.cpu().shares(max).done();
    }

    if let Some(run_request::Disk::MaxDisk(max)) = request.disk {
        builder = builder.blkio().weight(u16::try_from(max)?).done();
    }

    Ok(builder.build()?)
}
