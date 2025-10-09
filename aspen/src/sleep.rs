use std::{hint, time::Duration};

use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

/// 1µs precision sleep function
#[cfg(windows)]
pub fn u_sleep(dur: Duration) {
    #[cold]
    fn cold_path() {}

    let wait = dur.as_micros().clamp(u64::MIN as u128, u64::MAX as u128) as u64;

    let mut t1 = 0i64;
    let mut t2 = 0i64;
    let mut freq = 0i64;

    let res = unsafe { QueryPerformanceCounter(&mut t1) };
    if let Err(e) = res {
        cold_path();
        panic!("u_sleep QueryPerformanceCounter: {e:?}");
    }

    let res = unsafe { QueryPerformanceFrequency(&mut freq) };
    if let Err(e) = res {
        cold_path();
        panic!("u_sleep QueryPerformanceFrequency: {e:?}");
    }

    let target = wait * (freq as u64 / 1000000);

    loop {
        let res = unsafe { QueryPerformanceCounter(&mut t2) };
        if let Err(e) = res {
            cold_path();
            panic!("u_sleep loop QueryPerformanceCounter: {e:?}");
        }

        if (t2 as u64 - t1 as u64) > target {
            break;
        }

        hint::spin_loop();
    }
}

#[cfg(not(windows))]
pub fn u_sleep(dur: Duration) {
    unimplemented!()
}
