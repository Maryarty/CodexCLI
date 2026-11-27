//! The host reads a clipboard image into a file. Its answer belongs to the
//! draft that requested it, even if another thread is showing when it arrives.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::ids::{ClipboardRead, ThreadId};

/// The latest clipboard read of each thread, waiting on the host or on its composer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pasted {
    reads: BTreeMap<ThreadId, Read>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Read {
    number: u64,
    result: Option<Result<PathBuf, String>>,
}

impl Pasted {
    pub(in crate::screens) fn wait(&mut self, read: &ClipboardRead) {
        self.reads.insert(
            read.thread.clone(),
            Read {
                number: read.number,
                result: None,
            },
        );
    }

    pub(in crate::screens) fn answered(
        &mut self,
        read: &ClipboardRead,
        result: Result<PathBuf, String>,
    ) {
        if let Some(waiting) = self.reads.get_mut(&read.thread)
            && waiting.number == read.number
            && waiting.result.is_none()
        {
            waiting.result = Some(result);
        }
    }

    pub(in crate::screens) fn forget(&mut self, read: &ClipboardRead) {
        if self
            .reads
            .get(&read.thread)
            .is_some_and(|waiting| waiting.number == read.number)
        {
            self.reads.remove(&read.thread);
        }
    }

    pub fn result(&self, thread: &ThreadId, number: u64) -> Option<&Result<PathBuf, String>> {
        self.reads
            .get(thread)
            .filter(|read| read.number == number)?
            .result
            .as_ref()
    }
}
