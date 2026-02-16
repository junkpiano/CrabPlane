use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(unix)]
mod imp {
    use super::*;
    use std::ffi::c_int;

    // These are the conventional Linux values.
    const SIGINT: c_int = 2;
    const SIGTERM: c_int = 15;

    type Sighandler = extern "C" fn(c_int);

    unsafe extern "C" {
        fn signal(sig: c_int, handler: Sighandler) -> Sighandler;
    }

    static mut STOP_PTR: *const AtomicBool = std::ptr::null();

    extern "C" fn handler(_sig: c_int) {
        unsafe {
            if !STOP_PTR.is_null() {
                // Safety: STOP_PTR is set once at startup and lives for the process duration.
                let stop = &*STOP_PTR;
                stop.store(true, Ordering::Relaxed);
            }
        }
    }

    pub fn install(stop: &Arc<AtomicBool>) {
        unsafe {
            STOP_PTR = Arc::as_ptr(stop);
            let _ = signal(SIGINT, handler);
            let _ = signal(SIGTERM, handler);
        }
    }
}

#[cfg(not(unix))]
mod imp {
    use super::*;
    pub fn install(_stop: &Arc<AtomicBool>) {}
}

pub fn install_unix_signal_handlers(stop: &Arc<AtomicBool>) {
    imp::install(stop);
}
