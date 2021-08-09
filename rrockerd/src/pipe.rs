use anyhow::{Context, Result};
use nix::{fcntl::OFlag, unistd};
use std::{fs::File, os::unix::prelude::FromRawFd};

pub struct Pipe {
    reader: File,
    writer: File,
}

impl Pipe {
    pub fn new() -> Result<Self> {
        let (reader_fd, writer_fd) =
            unistd::pipe2(OFlag::O_CLOEXEC).context("Failed to call pipe2()")?;

        unsafe {
            Ok(Self {
                reader: File::from_raw_fd(reader_fd),
                writer: File::from_raw_fd(writer_fd),
            })
        }
    }

    pub fn split(self) -> (File, File) {
        (self.reader, self.writer)
    }
}
