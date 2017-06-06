//! Main module of rexpect: start new process and interact with it

use process::PtyProcess;
use std::io::{BufReader, LineWriter};
use std::ffi::OsStr;
use std::fs::File;
use std::process::Command;
use std::os::unix::io::{FromRawFd, AsRawFd};
use std::io::prelude::*;
use nix::sys::{wait, signal};
use nix::unistd;
use errors::*; // load error-chain

/// Interact with a process with read/write/signals, etc.
pub struct PtySession {
    process: PtyProcess,
    writer: LineWriter<File>,
    reader: BufReader<File>,
}

impl PtySession {
    pub fn send_line(&mut self, line: &str) -> Result<()> {
        self.writer.write_all(line.as_bytes()).chain_err(|| "cannot write line to process")
    }

    /// get status of child process, nonblocking
    ///
    /// # Example
    /// ```rust,no_run
    ///
    /// # extern crate nix;
    /// # extern crate rexpect;
    /// # use nix::sys::wait;
    ///
    /// # fn main() {
    ///     let process = rexpect::spawn("sleep 5").expect("cannot run cat");
    ///     while process.status().unwrap() == wait::WaitStatus::StillAlive {
    ///         // do something
    ///     }
    /// # }
    /// ```
    ///
    pub fn status(&self) -> Result<(wait::WaitStatus)> {
        wait::waitpid(self.process.child_pid, Some(wait::WNOHANG)).chain_err(|| "cannot read status")
    }

    /// regularly exit the process
    ///
    /// sends SIGHUP and closes the pty session
    pub fn exit(&self) -> Result<()> {
        signal::kill(self.process.child_pid, signal::SIGHUP).and_then(|_|
            unistd::close(self.process.pty.as_raw_fd())
        ).chain_err(|| "failed to exit process")
    }
}

pub fn spawn<S: AsRef<OsStr>>(program: S) -> Result<PtySession> {
    let command = Command::new(program);
    let process = PtyProcess::new(command).chain_err(|| "couldn't start process")?;
    let f = unsafe { File::from_raw_fd(process.pty.as_raw_fd()) };
    let writer = LineWriter::new(f.try_clone().chain_err(|| "couldn't open write stream")?);
    let reader = BufReader::new(f);
    Ok(PtySession {
           process: process,
           writer: writer,
           reader: reader,
       })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep_ms;
    #[test]
    fn test_cat() {
        || -> Result<()> {
            let mut s = spawn("cat")?;
            s.send_line("hans")?;
            s.exit()?;
            println!("status={:?}", s.status()?);
            Ok(())
        }().expect("could not execute");
    }

}