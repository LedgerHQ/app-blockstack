#![no_std]
#![no_builtins]
#![allow(dead_code, unused_imports)]
#![macro_use]

#[cfg(test)]
#[macro_use]
extern crate std;

mod bolos;
mod parser;
mod zxformat;

extern crate core;

fn debug(_msg: &str) {}

#[cfg(not(test))]
use core::panic::PanicInfo;

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

extern "C" {
    fn check_canary();
    fn pic(link_address: u32) -> u32;
    fn app_mode_expert() -> u8;
}

pub(crate) fn canary() {
    #[cfg(not(test))]
    unsafe {
        check_canary();
    }
}

pub fn pic_internal<T: Sized>(obj: &T) -> &T {
    if cfg!(test) {
        return obj;
    }
    let ptr = obj as *const _;
    let ptr_usize = ptr as *const () as u32;
    unsafe {
        let link = pic(ptr_usize);
        let ptr = link as *const T;
        &*ptr
    }
}

pub fn is_expert_mode() -> bool {
    if cfg!(test) {
        true
    } else {
        unsafe { app_mode_expert() > 0 }
    }
}

#[macro_export]
macro_rules! pic {
    ($obj:expr) => {{
        use crate::pic_internal;
        pic_internal(&$obj)
    }};
}

#[macro_export]
macro_rules! check_canary {
    () => {
        use crate::canary;
        canary();
    };
}
