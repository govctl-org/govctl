use std::process::{Child, Command};
use std::time::Duration;

#[cfg(unix)]
pub(super) fn configure_guard_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(not(unix))]
pub(super) fn configure_guard_process_group(_command: &mut Command) {}

#[cfg(unix)]
#[derive(Clone, Copy)]
pub(super) struct GuardProcessGroup {
    pgid: i32,
}

#[cfg(not(unix))]
#[derive(Clone, Copy)]
pub(super) struct GuardProcessGroup;

#[cfg(unix)]
pub(super) fn guard_process_group(child_id: u32) -> Option<GuardProcessGroup> {
    let child_pid = i32::try_from(child_id).ok()?;
    let pgid = unix_getpgid(child_pid)?;
    let current_pgid = unix_getpgrp()?;

    if pgid == current_pgid || pgid != child_pid {
        return None;
    }

    Some(GuardProcessGroup { pgid })
}

#[cfg(not(unix))]
pub(super) fn guard_process_group(_child_id: u32) -> Option<GuardProcessGroup> {
    None
}

pub(super) fn terminate_guard_process(child: &mut Child, process_group: Option<GuardProcessGroup>) {
    terminate_guard_process_group(process_group);
    let _ = child.kill();
}

#[cfg(unix)]
pub(super) fn terminate_guard_process_group(process_group: Option<GuardProcessGroup>) {
    let Some(process_group) = process_group else {
        return;
    };

    signal_process_group(process_group, SIGTERM);
    std::thread::sleep(Duration::from_millis(25));
    signal_process_group(process_group, SIGKILL);
}

#[cfg(unix)]
fn signal_process_group(process_group: GuardProcessGroup, signal: i32) {
    // Negative PID targets the process group. The group is captured after spawn
    // and rejected if it is not the isolated child group.
    unsafe {
        let _ = kill(-process_group.pgid, signal);
    }
}

#[cfg(not(unix))]
pub(super) fn terminate_guard_process_group(_process_group: Option<GuardProcessGroup>) {}

#[cfg(unix)]
const SIGTERM: i32 = 15;

#[cfg(unix)]
const SIGKILL: i32 = 9;

#[cfg(unix)]
fn unix_getpgid(pid: i32) -> Option<i32> {
    let pgid = unsafe { getpgid(pid) };
    (pgid > 0).then_some(pgid)
}

#[cfg(unix)]
fn unix_getpgrp() -> Option<i32> {
    let pgid = unsafe { getpgrp() };
    (pgid > 0).then_some(pgid)
}

#[cfg(unix)]
unsafe extern "C" {
    fn getpgid(pid: i32) -> i32;
    fn getpgrp() -> i32;
    fn kill(pid: i32, sig: i32) -> i32;
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn current_process_group_is_not_treated_as_guard_group() {
        assert!(guard_process_group(std::process::id()).is_none());
    }

    #[test]
    fn isolated_child_process_group_is_captured() -> Result<(), Box<dyn std::error::Error>> {
        let mut command = Command::new("/bin/sleep");
        command.arg("5");
        configure_guard_process_group(&mut command);

        let mut child = command.spawn()?;
        let process_group = guard_process_group(child.id());

        assert!(process_group.is_some());
        terminate_guard_process(&mut child, process_group);
        let _ = child.wait();

        Ok(())
    }
}
