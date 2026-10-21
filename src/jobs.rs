//! Worker progress and cooperative cancellation. Publication and cancellation share a gate.
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    sync::{Mutex, OnceLock},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Progress {
    pub completed: usize,
    pub total: usize,
    pub item: String,
    #[serde(default)]
    pub fraction: Option<f64>,
}

#[derive(Default)]
pub struct Cancellation(Mutex<bool>);
impl Cancellation {
    pub fn cancel(&self) {
        *self.0.lock().unwrap() = true;
    }
    pub fn check(&self) -> Result<(), String> {
        if *self.0.lock().unwrap() {
            Err("Cancelled. Previously completed outputs are kept.".into())
        } else {
            Ok(())
        }
    }
    pub fn publish<T>(&self, write: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
        let cancelled = self.0.lock().unwrap();
        if *cancelled {
            return Err("Cancelled. Previously completed outputs are kept.".into());
        }
        write()
    }
}
static WORKER: OnceLock<Cancellation> = OnceLock::new();
pub fn worker_start() {
    let _ = WORKER.set(Cancellation::default());
}
pub fn cancel() {
    if let Some(c) = WORKER.get() {
        c.cancel();
    }
}
pub fn check() -> Result<(), String> {
    WORKER.get().map_or(Ok(()), Cancellation::check)
}
pub fn publish<T>(write: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    if let Some(c) = WORKER.get() {
        c.publish(write)
    } else {
        write()
    }
}
pub fn progress(completed: usize, total: usize, item: impl Into<String>) {
    progress_fraction(completed, total, item, None);
}
pub fn progress_fraction(
    completed: usize,
    total: usize,
    item: impl Into<String>,
    fraction: Option<f64>,
) {
    if WORKER.get().is_some() {
        let item: String = item.into().chars().take(180).collect();
        if let Ok(data) = serde_json::to_string(&Progress {
            completed,
            total,
            item,
            fraction,
        }) {
            let _ = writeln!(std::io::stderr().lock(), "{data}");
        }
    }
}

/// Keep parsers/codecs and any descendants bounded and owned by the caller.
#[cfg(windows)]
pub struct WorkerJob(windows_sys::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl WorkerJob {
    pub fn attach(child: &std::process::Child) -> Result<Self, String> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::JobObjects::*;
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err(std::io::Error::last_os_error().to_string());
            }
            let job = Self(handle);
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
                | JOB_OBJECT_LIMIT_JOB_MEMORY
                | JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
            limits.BasicLimitInformation.ActiveProcessLimit = 16;
            limits.JobMemoryLimit = 1024 * 1024 * 1024;
            if SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as _,
                std::mem::size_of_val(&limits) as u32,
            ) == 0
                || AssignProcessToJobObject(handle, child.as_raw_handle() as _) == 0
            {
                return Err(format!(
                    "Cannot contain tool worker: {}",
                    std::io::Error::last_os_error()
                ));
            }
            Ok(job)
        }
    }
    pub fn terminate(&self) {
        unsafe {
            windows_sys::Win32::System::JobObjects::TerminateJobObject(self.0, 1);
        }
    }
}
#[cfg(windows)]
impl Drop for WorkerJob {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_preserves_completed_output_and_blocks_later_publication() {
        let c = Cancellation::default();
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("result");
        c.publish(|| std::fs::write(&p, b"completed").map_err(|e| e.to_string()))
            .unwrap();
        c.cancel();
        assert!(c
            .publish(|| std::fs::write(&p, b"replaced").map_err(|e| e.to_string()))
            .is_err());
        assert_eq!(std::fs::read(&p).unwrap(), b"completed");
        assert!(c.check().is_err());
    }
}
