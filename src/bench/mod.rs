use std::time::Instant;

pub struct StageTimer {
    started_at: Instant,
}

impl StageTimer {
    pub fn start() -> Self {
        Self {
            started_at: Instant::now(),
        }
    }

    pub fn elapsed_ms(&self) -> u128 {
        self.started_at.elapsed().as_millis()
    }
}

/// Restore the default SIGPIPE handler so that piping output to `head`
/// or similar tools terminates cleanly instead of panicking.
pub fn restore_sigpipe() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

/// Raise the soft file descriptor limit toward the hard limit.
/// External merge sort opens many segment files concurrently.
pub fn raise_fd_limit() {
    unsafe {
        let mut rlim = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        if libc::getrlimit(libc::RLIMIT_NOFILE, &mut rlim) == 0 && rlim.rlim_cur < rlim.rlim_max {
            rlim.rlim_cur = rlim.rlim_max;
            libc::setrlimit(libc::RLIMIT_NOFILE, &rlim);
        }
    }
}

/// Return peak resident set size in bytes, or `None` if unavailable.
pub fn peak_rss_bytes() -> Option<u64> {
    unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        if libc::getrusage(libc::RUSAGE_SELF, &mut usage) == 0 {
            // On macOS, ru_maxrss is in bytes.
            // On Linux, ru_maxrss is in kilobytes.
            let rss = usage.ru_maxrss;
            if rss > 0 {
                #[cfg(target_os = "macos")]
                {
                    return Some(rss as u64);
                }
                #[cfg(not(target_os = "macos"))]
                {
                    return Some(rss as u64 * 1024);
                }
            }
        }
        None
    }
}
