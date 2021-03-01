use crate::runner::process_map::ProcessMap;
use crate::runner::service::log_response::LogError;
use anyhow::{anyhow, Context, Result};
use futures::stream::Stream;
use futures::task::Poll;
use std::borrow::BorrowMut;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::RwLock;

/// A struct representing the stream of stdout or stderr data of a process.
/// This struct implements futures::stream::Stream as well as some common traits
#[derive(Clone, Debug)]
pub struct LogStream {
    /// A processes map providing information on whether a process is still running or not
    map: ProcessMap,

    /// A log file
    file: Arc<RwLock<File>>,

    /// UUID of the process
    process_id: String,

    /// Internal state variable telling if reading can continue
    closed: bool,
}

impl LogStream {
    /// Creates a stream of messages, ready to be polled for new data
    pub fn open(process_id: String, map: ProcessMap, path: &Path) -> Result<Self> {
        let file = Arc::new(RwLock::new(
            File::open(&path).context("Couldn't open log file")?,
        ));

        Ok(LogStream {
            map,
            file,
            process_id,
            closed: false,
        })
    }
}

impl Stream for LogStream {
    type Item = Result<Vec<u8>, LogError>;

    /// Returns the next value from the log stream. If the process has finished, then
    /// reaching the end of file finishes this stream.
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
