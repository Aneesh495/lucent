use std::{
    cmp::Ordering,
    ffi::{CStr, CString, c_char, c_int, c_void},
    ptr::NonNull,
};

unsafe extern "C" {
    fn lc_mp_new(bits: u32) -> *mut c_void;
    fn lc_mp_drop(n: *mut c_void);
    fn lc_mp_set_double(out: *mut c_void, n: f64, up: c_int) -> c_int;
    fn lc_mp_set_ratio(out: *mut c_void, n: *const c_char, up: c_int) -> c_int;
    fn lc_mp_get(n: *const c_void, up: c_int) -> f64;
    fn lc_mp_compare(a: *const c_void, b: *const c_void) -> c_int;
    fn lc_mp_sign(a: *const c_void) -> c_int;
    fn lc_mp_copy(out: *mut c_void, n: *const c_void, up: c_int) -> c_int;
    fn lc_mp_binary(
        out: *mut c_void,
        a: *const c_void,
        b: *const c_void,
        op: c_int,
        up: c_int,
    ) -> c_int;
    fn lc_mp_unary(out: *mut c_void, a: *const c_void, op: c_int, up: c_int) -> c_int;
    fn lc_mp_pow(out: *mut c_void, a: *const c_void, n: i32, up: c_int) -> c_int;
    fn lc_mp_root(out: *mut c_void, a: *const c_void, degree: u32, up: c_int) -> c_int;
    fn lc_mp_pi(out: *mut c_void, up: c_int) -> c_int;
    fn lc_mp_has_integer(a: *const c_void, b: *const c_void) -> c_int;
    fn lc_mp_version() -> *const c_char;
}

#[derive(Clone, Copy, Debug)]
pub enum Direction {
    Down,
    Up,
}

#[derive(Clone, Copy, Debug)]
pub enum Binary {
    Add,
    Sub,
    Mul,
    Div,
    Atan2,
}

#[derive(Clone, Copy, Debug)]
pub enum Unary {
    Neg,
    Sqrt,
    Exp,
    Log,
    Sin,
    Cos,
    Tan,
    Atan,
    Abs,
    Erf,
    Erfc,
}

#[derive(Debug, Clone, Copy)]
pub struct RangeError;
type Result<T> = std::result::Result<T, RangeError>;

pub struct Float {
    raw: NonNull<c_void>,
    bits: u32,
}

impl Float {
    fn write(bits: u32, operation: impl FnOnce(*mut c_void) -> c_int) -> Result<Self> {
        if !(53..=512).contains(&bits) {
            return Err(RangeError);
        }
        let raw = NonNull::new(unsafe { lc_mp_new(bits) }).ok_or(RangeError)?;
        let value = Self { raw, bits };
        if operation(raw.as_ptr()) == 0 {
            return Err(RangeError);
        }
        Ok(value)
    }

    fn ptr(&self) -> *const c_void {
        self.raw.as_ptr()
    }
    pub fn bits(&self) -> u32 {
        self.bits
    }

    pub fn from_f64(x: f64, bits: u32, round: Direction) -> Result<Self> {
        if !x.is_finite() {
            return Err(RangeError);
        }
        Self::write(bits, |out| unsafe { lc_mp_set_double(out, x, round as _) })
    }

    pub fn ratio(x: &str, bits: u32, round: Direction) -> Result<Self> {
        let x = CString::new(x).map_err(|_| RangeError)?;
        Self::write(bits, |out| unsafe {
            lc_mp_set_ratio(out, x.as_ptr(), round as _)
        })
    }

    pub fn pi(bits: u32, round: Direction) -> Result<Self> {
        Self::write(bits, |out| unsafe { lc_mp_pi(out, round as _) })
    }

    pub fn to_f64(&self, round: Direction) -> f64 {
        unsafe { lc_mp_get(self.ptr(), round as _) }
    }

    pub fn compare(&self, other: &Self) -> Ordering {
        unsafe { lc_mp_compare(self.ptr(), other.ptr()) }.cmp(&0)
    }

    pub fn sign(&self) -> Ordering {
        unsafe { lc_mp_sign(self.ptr()) }.cmp(&0)
    }

    pub fn binary(&self, other: &Self, op: Binary, round: Direction) -> Result<Self> {
        Self::write(self.bits, |out| unsafe {
            lc_mp_binary(out, self.ptr(), other.ptr(), op as _, round as _)
        })
    }

    pub fn unary(&self, op: Unary, round: Direction) -> Result<Self> {
        Self::write(self.bits, |out| unsafe {
            lc_mp_unary(out, self.ptr(), op as _, round as _)
        })
    }

    pub fn pow(&self, exponent: i32, round: Direction) -> Result<Self> {
        Self::write(self.bits, |out| unsafe {
            lc_mp_pow(out, self.ptr(), exponent, round as _)
        })
    }

    pub fn root(&self, degree: u32, round: Direction) -> Result<Self> {
        Self::write(self.bits, |out| unsafe {
            lc_mp_root(out, self.ptr(), degree, round as _)
        })
    }

    pub fn has_integer_through(&self, other: &Self) -> bool {
        unsafe { lc_mp_has_integer(self.ptr(), other.ptr()) != 0 }
    }
}

impl Clone for Float {
    fn clone(&self) -> Self {
        Self::write(self.bits, |out| unsafe { lc_mp_copy(out, self.ptr(), 0) })
            .expect("copy a valid MPFR value")
    }
}

impl Drop for Float {
    fn drop(&mut self) {
        unsafe { lc_mp_drop(self.raw.as_ptr()) }
    }
}

pub fn version() -> &'static str {
    unsafe { CStr::from_ptr(lc_mp_version()) }
        .to_str()
        .expect("ASCII MPFR version")
}
