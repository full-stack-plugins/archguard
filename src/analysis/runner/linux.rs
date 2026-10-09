//! Inherited process-group confinement for trusted fixed tools, not a general sandbox.
use std::io;
fn stmt(code: u16, k: u32) -> libc::sock_filter {
    libc::sock_filter {
        code,
        jt: 0,
        jf: 0,
        k,
    }
}
fn equal(k: u32, jt: u8, jf: u8) -> libc::sock_filter {
    libc::sock_filter {
        code: 0x15,
        jt,
        jf,
        k,
    }
}
pub(super) fn filter() -> Vec<libc::sock_filter> {
    #[cfg(target_arch = "x86_64")]
    const ARCH: u32 = 0xc000003e;
    #[cfg(target_arch = "aarch64")]
    const ARCH: u32 = 0xc00000b7;
    let mut code = vec![
        stmt(0x20, 4),
        equal(ARCH, 1, 0),
        stmt(0x06, 0x80000000),
        stmt(0x20, 0),
    ];
    #[cfg(target_arch = "x86_64")]
    code.extend([
        libc::sock_filter {
            code: 0x35,
            jt: 0,
            jf: 1,
            k: 0x40000000,
        },
        stmt(0x06, 0x80000000),
    ]);
    for nr in [
        libc::SYS_setsid,
        libc::SYS_setpgid,
        libc::SYS_unshare,
        libc::SYS_setns,
    ] {
        code.extend([
            equal(nr as u32, 0, 1),
            stmt(0x06, 0x00050000 | libc::EPERM as u32),
        ]);
    }
    // clone3 flags live behind a userspace pointer. Refuse it; libc can fall back
    // to clone, whose namespace flags we can inspect without reading pointers.
    code.extend([
        equal(libc::SYS_clone3 as u32, 0, 1),
        stmt(0x06, 0x00050000 | libc::ENOSYS as u32),
    ]);
    let namespaces = libc::CLONE_NEWCGROUP
        | libc::CLONE_NEWIPC
        | libc::CLONE_NEWNET
        | libc::CLONE_NEWNS
        | libc::CLONE_NEWPID
        | libc::CLONE_NEWUSER
        | libc::CLONE_NEWUTS;
    code.extend([
        equal(libc::SYS_clone as u32, 0, 3),
        stmt(0x20, 16), // seccomp_data.args[0], low word (supported little-endian ABIs)
        libc::sock_filter {
            code: 0x45,
            jt: 0,
            jf: 1,
            k: namespaces as u32,
        },
        stmt(0x06, 0x00050000 | libc::EPERM as u32),
        stmt(0x06, 0x7fff0000),
    ]);
    code
}
/// Called only in Command::pre_exec: all allocations happened in the parent.
pub(super) fn install(code: &[libc::sock_filter]) -> io::Result<()> {
    let program = libc::sock_fprog {
        len: code.len() as u16,
        filter: code.as_ptr().cast_mut(),
    };
    // SAFETY: valid stack program and borrowed filter remain alive for these
    // synchronous kernel copies. prctl changes only this child and descendants.
    unsafe {
        if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0
            || libc::prctl(libc::PR_SET_SECCOMP, 2, &program) != 0
        {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}
/// Preserve the leader zombie/PID until the entire process group is signalled.
pub(super) fn exited(pid: u32) -> io::Result<bool> {
    // SAFETY: zero initialization is valid for siginfo_t; waitid writes it.
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    if unsafe {
        libc::waitid(
            libc::P_PID,
            pid,
            &mut info,
            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: waitid initializes the SIGCHLD union or leaves zero when no event.
    Ok(unsafe { info.si_pid() } != 0)
}
