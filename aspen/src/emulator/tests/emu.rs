use std::{
    ops::{Deref, DerefMut},
    sync::{LazyLock, Mutex, MutexGuard},
};

use aho_corasick::AhoCorasick;

use super::{EmuError, Emulator};
use crate::memory::Prot;

#[derive(Debug)]
pub struct EmuGuard<'a>(MutexGuard<'a, Emulator>, bool);

impl Deref for EmuGuard<'_> {
    type Target = Emulator;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for EmuGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for EmuGuard<'_> {
    fn drop(&mut self) {
        self.cpu.zeroize();
        // mem dirty flag
        let dirty = self.1;
        // skip mem resetting if there's nothing to reset, to save on processing
        let mem = self.mem.get_mut().unwrap();
        if dirty {
            mem.zeroize().expect("zeroize to succeed");
        }
        mem.change_prot(.., Prot::Read | Prot::Write).unwrap();
    }
}

#[doc(hidden)]
pub fn _try_run(asm: &str) -> Result<EmuGuard<'_>, EmuError> {
    _try_run_with(|_| (), asm)
}

#[doc(hidden)]
pub fn _try_run_with(f: impl FnOnce(&mut Emulator), asm: &str) -> Result<EmuGuard<'_>, EmuError> {
    static LOCK: LazyLock<Mutex<Emulator>> =
        LazyLock::new(|| Mutex::new(Emulator::new(&[]).unwrap()));

    LOCK.clear_poison();

    let patterns = &["str", "str.w", "str.b"];
    let ac = AhoCorasick::new(patterns).unwrap();
    let dirty = ac.find(asm).is_some();

    // important. this will keep it synchronous
    let mut guard = EmuGuard(LOCK.lock().unwrap(), dirty);

    let asm = format!("{asm}\n\n; auto inserted\nhlt");

    let data = match graft::assemble("<input>.asm", &asm) {
        Ok(d) => d,
        Err(e) => panic!("{e}"),
    };

    guard.write_program(&data)?;

    f(&mut guard);

    guard.run()?;

    Ok(guard)
}

pub mod macros {
    macro_rules! try_run_with {
        (|$d:ident| {
            $($ccode:tt)*
        },
        $($code:tt)*) => {
            _try_run_with(|$d| { $($ccode)* }, ::kizuna::macros::stringify_raw!($($code)*))
        };

        (
            $d:ident,
            $($code:tt)*
        ) => {
            $crate::emulator::tests::emu::_try_run_with($d, ::kizuna::macros::stringify_raw!($($code)*))
        };
    }

    #[expect(unused)]
    macro_rules! try_run {
        ($($code:tt)*) => {
            _try_run(::kizuna::macros::stringify_raw!($($code)*))
        };
    }

    macro_rules! run {
        ($($code:tt)*) => {
            $crate::emulator::tests::emu::_try_run(::kizuna::macros::stringify_raw!($($code)*)).unwrap()
        };
    }

    #[expect(unused)]
    pub(crate) use {run, try_run, try_run_with};
}
