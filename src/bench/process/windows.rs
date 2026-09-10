//! Windows process containment and read-only TCP port-state checks.

use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use tokio::process::{Child, Command};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Threading::*;

/// Windows job ownership makes cancellation close the entire child process tree.
pub(super) struct WindowsJob(std::os::windows::io::OwnedHandle);

impl WindowsJob {
    /// Creates a child that cannot execute before containment is configured.
    pub(super) fn spawn_suspended(mut command: Command) -> io::Result<Child> {
        // No child code can run or create descendants before job assignment.
        // Stable Rust exposes creation_flags but not the main thread handle.
        command
            .creation_flags(CREATE_SUSPENDED)
            .kill_on_drop(true)
            .spawn()
    }

    /// Resumes the suspended initial thread after this job has been attached.
    pub(super) fn resume(&self, child: &Child) -> io::Result<()> {
        use windows_sys::Win32::System::Diagnostics::ToolHelp::*;
        let pid = child
            .id()
            .ok_or_else(|| io::Error::other("suspended child exited"))?;
        // SAFETY: snapshot has no borrowed inputs; the valid handle is owned.
        let raw = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
        if raw == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        let snapshot = unsafe { OwnedHandle::from_raw_handle(raw) };
        let mut entry = THREADENTRY32 {
            dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
            ..Default::default()
        };
        // SAFETY: entry has the required initialized size, and snapshot is live.
        let mut found = unsafe { Thread32First(snapshot.as_raw_handle(), &mut entry) };
        while found != 0 {
            if entry.th32OwnerProcessID == pid {
                // CREATE_SUSPENDED has not run any user code: the process has
                // its initial thread only. Open by id while the child is alive.
                let raw = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID) };
                if raw.is_null() {
                    return Err(io::Error::last_os_error());
                }
                let thread = unsafe { OwnedHandle::from_raw_handle(raw) };
                // SAFETY: this is our suspended child's initial thread. The job
                // is already configured and attached before it can execute.
                if unsafe { ResumeThread(thread.as_raw_handle()) } == u32::MAX {
                    return Err(io::Error::last_os_error());
                }
                return Ok(());
            }
            found = unsafe { Thread32Next(snapshot.as_raw_handle(), &mut entry) };
        }
        Err(io::Error::other(
            "suspended child's initial thread was not found",
        ))
    }

    /// Configures kill-on-close ownership and assigns the suspended child.
    pub(super) fn attach(child: &Child) -> std::io::Result<Self> {
        use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
        use windows_sys::Win32::System::JobObjects::*;
        // SAFETY: null attributes/name request an unnamed job; the returned
        // handle is checked and transferred exactly once into OwnedHandle.
        let raw = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if raw.is_null() {
            return Err(std::io::Error::last_os_error());
        }
        let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: limits is the documented structure and size for this class;
        // both handles remain live across configuration and assignment.
        let configured = unsafe {
            SetInformationJobObject(
                handle.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limits) as u32,
            )
        };
        if configured == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let process = child
            .raw_handle()
            .ok_or_else(|| std::io::Error::other("child exited before job assignment"))?;
        if unsafe { AssignProcessToJobObject(handle.as_raw_handle(), process) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Self(handle))
    }

    /// Terminates every process in this owned job.
    pub(super) fn terminate(&self) {
        use std::os::windows::io::AsRawHandle;
        // SAFETY: this owned job contains only this verifier's process tree.
        unsafe {
            windows_sys::Win32::System::JobObjects::TerminateJobObject(self.0.as_raw_handle(), 1);
        }
    }
}

/// A bounded snapshot of the TCP table, retaining only initialized aligned bytes.
fn tcp_table(mut read: impl FnMut(*mut std::ffi::c_void, &mut u32) -> u32) -> io::Result<Vec<u32>> {
    let mut size = 0;
    let mut storage = Vec::<u32>::new();
    for _ in 0..3 {
        let ptr = if storage.is_empty() {
            std::ptr::null_mut()
        } else {
            storage.as_mut_ptr().cast()
        };
        match read(ptr, &mut size) {
            NO_ERROR => return Ok(storage),
            ERROR_INSUFFICIENT_BUFFER if (4..=1024 * 1024).contains(&size) => {
                storage.resize((size as usize).div_ceil(4), 0);
                size = (storage.len() * 4) as u32;
            }
            code => return Err(io::Error::from_raw_os_error(code as i32)),
        }
    }
    Err(io::Error::other(
        "TCP table changed beyond the bounded snapshot retries",
    ))
}

fn rows<T>(storage: &[u32], offset: usize) -> io::Result<&[T]> {
    let count = storage
        .first()
        .copied()
        .ok_or_else(|| io::Error::other("missing TCP table header"))? as usize;
    let bytes = std::mem::size_of_val(storage);
    if std::mem::align_of::<T>() > std::mem::align_of::<u32>()
        || !offset.is_multiple_of(std::mem::align_of::<T>())
        || offset > bytes
        || count > (bytes - offset) / std::mem::size_of::<T>()
    {
        return Err(io::Error::other("invalid TCP table bounds"));
    }
    // SAFETY: only POD TCP rows are requested below. GetTcp[6]Table initialized
    // the buffer, and alignment, offset and complete row bounds are checked.
    Ok(unsafe {
        std::slice::from_raw_parts(storage.as_ptr().cast::<u8>().add(offset).cast(), count)
    })
}

/// Checks loopback availability, permitting stale TIME_WAIT but no live endpoints.
pub(super) fn check_port_available(port: u16) -> io::Result<()> {
    use windows_sys::Win32::NetworkManagement::IpHelper::*;
    let socket = tokio::net::TcpSocket::new_v4()?;
    let address = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let conflict = match socket.bind(address) {
        Ok(()) => return Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AddrInUse => error,
        Err(error) => return Err(error),
    };
    // A reuse-enabled bind alone would permit taking over a Windows listener.
    // Allow it only when the current tables identify stale TIME_WAIT entries
    // and no active endpoint at this port (including potential IPv6 dual-stack).
    let v4 = tcp_table(|ptr, size| unsafe { GetTcpTable(ptr.cast(), size, 0) })?;
    let v6 = tcp_table(|ptr, size| unsafe { GetTcp6Table(ptr.cast(), size, 0) })?;
    let mut stale = false;
    for row in rows::<MIB_TCPROW_LH>(&v4, std::mem::offset_of!(MIB_TCPTABLE, table))? {
        if u16::from_be(row.dwLocalPort as u16) == port {
            if unsafe { row.Anonymous.State } != MIB_TCP_STATE_TIME_WAIT {
                return Err(conflict);
            }
            stale = true;
        }
    }
    for row in rows::<MIB_TCP6ROW>(&v6, std::mem::offset_of!(MIB_TCP6TABLE, table))? {
        if u16::from_be(row.dwLocalPort as u16) == port && row.State != MIB_TCP_STATE_TIME_WAIT {
            return Err(conflict);
        }
    }
    if !stale {
        return Err(conflict);
    }
    socket.set_reuseaddr(true)?;
    socket.bind(address)
}

/// Read-only child existence check for cancellation tests.
#[cfg(test)]
pub(super) fn process_alive(pid: u32) -> bool {
    // SAFETY: read-only process query. No process can be killed through this handle.
    let raw = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if raw.is_null() {
        return false;
    }
    let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
    let mut code = 0;
    unsafe {
        GetExitCodeProcess(handle.as_raw_handle(), &mut code) != 0 && code == STILL_ACTIVE as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn port_probe_distinguishes_live_listener_from_time_wait() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let socket = tokio::net::TcpSocket::new_v4().expect("socket");
        socket.set_reuseaddr(true).expect("runtime socket option");
        socket.bind(([127, 0, 0, 1], 0).into()).expect("bind");
        let listener = socket.listen(1).expect("listen");
        let address = listener.local_addr().expect("address");
        assert!(
            check_port_available(address.port()).is_err(),
            "must not take over a live listener"
        );
        let mut client = tokio::net::TcpStream::connect(address)
            .await
            .expect("client");
        let (mut server, _) = listener.accept().await.expect("accept");
        server.write_all(b"response").await.expect("response");
        server.shutdown().await.expect("server closes first");
        drop(server);
        drop(listener);
        let mut body = Vec::new();
        client
            .read_to_end(&mut body)
            .await
            .expect("response and EOF");
        assert_eq!(body, b"response");
        drop(client);
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if check_port_available(address.port()).is_ok() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("closed connection may leave TIME_WAIT, not an active listener");
    }

    #[tokio::test]
    async fn suspended_child_cannot_spawn_before_job_assignment() {
        let tmp = tempfile::tempdir().expect("workspace");
        let marker = tmp.path().join("descendant.pid");
        let mut command = Command::new(std::env::current_exe().expect("fixture binary"));
        command
            .args([
                "--exact",
                "bench::process::tests::process_child_fixture",
                "--nocapture",
            ])
            .env("DUUMBI_780_CHILD_MODE", "tree")
            .env("DUUMBI_780_PID_FILE", &marker);
        let mut child = WindowsJob::spawn_suspended(command).expect("suspended child");
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!marker.exists(), "child code ran before job assignment");
        let job = WindowsJob::attach(&child).expect("job attachment");
        job.resume(&child).expect("resume");
        let descendant: u32 = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Ok(text) = tokio::fs::read_to_string(&marker).await
                    && let Ok(pid) = text.parse()
                {
                    break pid;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("child ran after assignment");
        // Actual descendant membership proves containment precedes execution.
        let raw = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, descendant) };
        assert!(!raw.is_null());
        let process = unsafe { OwnedHandle::from_raw_handle(raw) };
        let mut assigned = 0;
        assert_ne!(
            unsafe {
                windows_sys::Win32::System::JobObjects::IsProcessInJob(
                    process.as_raw_handle(),
                    job.0.as_raw_handle(),
                    &mut assigned,
                )
            },
            0
        );
        assert_ne!(assigned, 0);
        job.terminate();
        tokio::time::timeout(Duration::from_secs(2), child.wait())
            .await
            .expect("cleanup")
            .expect("wait");
    }
}
