use serde::Deserialize;
use serde_json::{Value, json};

use crate::mp::{
    self,
    Direction::{Down, Up},
    Float, Unary,
};

const REQUEST_LIMIT: usize = 65_536;

#[derive(Deserialize)]
struct Probe {
    ratio: String,
    bits: u32,
}

fn probe(input: Probe) -> Option<Value> {
    let Probe { ratio, bits } = input;
    let lo = Float::ratio(&ratio, bits, Down)
        .ok()?
        .unary(Unary::Exp, Down)
        .ok()?
        .to_f64(Down);
    let hi = Float::ratio(&ratio, bits, Up)
        .ok()?
        .unary(Unary::Exp, Up)
        .ok()?
        .to_f64(Up);
    if !lo.is_finite() || !hi.is_finite() {
        return None;
    }
    Some(json!({"status":"enclosure","bounds":[lo,hi],"precision":bits,"mpfr":mp::version()}))
}

pub fn request(input: &[u8]) -> Vec<u8> {
    let output = if input.len() > REQUEST_LIMIT {
        json!({"status":"error","message":"Request exceeds 64 KiB"})
    } else {
        match serde_json::from_slice::<Probe>(input) {
            Ok(parsed) => probe(parsed).unwrap_or_else(
                || json!({"status":"unresolved","message":"Invalid or unbounded probe"}),
            ),
            Err(error) => json!({"status":"error","message":error.to_string()}),
        }
    };
    serde_json::to_vec(&output).expect("JSON result")
}

#[cfg(target_os = "emscripten")]
mod abi {
    use super::{REQUEST_LIMIT, request};
    use std::cell::RefCell;

    thread_local! {
        static OUTPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    }

    #[unsafe(no_mangle)]
    extern "C" fn lc_alloc(len: usize) -> *mut u8 {
        if len == 0 || len > REQUEST_LIMIT {
            return std::ptr::null_mut();
        }
        Box::into_raw(vec![0u8; len].into_boxed_slice()).cast()
    }

    #[unsafe(no_mangle)]
    unsafe extern "C" fn lc_free(ptr: *mut u8, len: usize) {
        // The caller returns the allocation once, with its original length.
        if !ptr.is_null() {
            unsafe { drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len))) };
        }
    }

    #[unsafe(no_mangle)]
    unsafe extern "C" fn lc_run(ptr: *const u8, len: usize) -> *const u8 {
        // The worker owns the input allocation until this synchronous call returns.
        let input = if ptr.is_null() || len > REQUEST_LIMIT {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(ptr, len) }
        };
        OUTPUT.with_borrow_mut(|output| {
            *output = request(input);
            output.as_ptr()
        })
    }

    #[unsafe(no_mangle)]
    extern "C" fn lc_output_len() -> usize {
        OUTPUT.with_borrow(Vec::len)
    }
}
