use crate::runner::process_map::ProcessMap;
use crate::runner::service::log_response::LogError;
use anyhow::{anyhow, Context, Result};
use futures::stream::Stream;
use futures::task::Poll;
use std::borrow::BorrowMut;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::RwLock;

pub struct LogStream {
    map: ProcessMap,
    file: Arc<RwLock<File>>,
    process_id: String,
    closed: bool,
}

impl LogStream {
    pub fn open(process_id: String, processes: ProcessMap, path: PathBuf) -> Result<Self> {
        let file = File::open(&path).context("Couldn't open log file")?;

        Ok(LogStream {
            map: processes,
            file: Arc::new(RwLock::new(file)),
            process_id: process_id,
            closed: false,
        })
    }
}

impl Stream for LogStream {
    type Item = Result<Vec<u8>, LogError>;

    fn poll_next(
        self: Pin<&mut Self>,
        _cx: &mut futures::task::Context,
    ) -> Poll<Option<Self::Item>> {
        if self.closed {
            return Poll::Ready(None);
        }

        let this = Pin::<&mut LogStream>::into_inner(self);

        if let Some((_, Some(_))) = this.map.read().unwrap().get(&this.process_id) {
            // exit code is present, process has ended, we're finishing here
            if this.closed {
                return Poll::Ready(None);
            }
        }

        let mut buffer = [0; 32];

        let mut file_lock = this.file.write().unwrap();
        let file = file_lock.borrow_mut();

        if let Ok(bytes) = file.read(&mut buffer) {
            if bytes > 0 {
                Poll::Ready(Some(Ok(buffer[0..bytes].to_vec())))
            } else {
                // looks like there's no new data for now
                Poll::Pending
            }
        } else {
            this.closed = true;

            Poll::Ready(Some(Err(anyhow!("Error reading from log file").into())))
        }
    }
}
