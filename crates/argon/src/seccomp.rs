use seccompiler::{BpfProgram, SeccompAction, SeccompFilter, TargetArch};

#[cfg(target_arch = "x86_64")]
const ARCH: TargetArch = TargetArch::x86_64;
#[cfg(target_arch = "aarch64")]
const ARCH: TargetArch = TargetArch::aarch64;
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
compile_error!("supports only x86_64 and aarch64");

#[cfg(not(target_os = "linux"))]
compile_error!("supports only linux");

pub fn setup_seccomp(ungron: bool) {
    let mut rules = vec![(libc::SYS_write, vec![])];
    rules.extend_from_slice(&[
        (libc::SYS_exit_group, vec![]),
        (libc::SYS_ioctl, vec![]),
        (libc::SYS_madvise, vec![]),
        (libc::SYS_munmap, vec![]),
        (libc::SYS_sigaltstack, vec![]),
    ]);
    if ungron {
        rules.extend_from_slice(&[
            (libc::SYS_clone3, vec![]),
            (libc::SYS_close, vec![]),
            (libc::SYS_futex, vec![]),
            (libc::SYS_getrandom, vec![]),
            (libc::SYS_lseek, vec![]),
            (libc::SYS_mmap, vec![]),
            (libc::SYS_mprotect, vec![]),
            (libc::SYS_openat, vec![]),
            (libc::SYS_read, vec![]),
            (libc::SYS_rseq, vec![]),
            (libc::SYS_rt_sigaction, vec![]),
            (libc::SYS_rt_sigprocmask, vec![]),
            (libc::SYS_sched_getaffinity, vec![]),
            (libc::SYS_sched_yield, vec![]),
            (libc::SYS_set_robust_list, vec![]),
            (libc::SYS_statx, vec![]),
        ]);
    }

    let bpf_prog: BpfProgram = SeccompFilter::new(
        rules.into_iter().collect(),
        SeccompAction::Trap,
        SeccompAction::Allow,
        ARCH,
    )
    .unwrap()
    .try_into()
    .unwrap();

    seccompiler::apply_filter(&bpf_prog).unwrap();
}
