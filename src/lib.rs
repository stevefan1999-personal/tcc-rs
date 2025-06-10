#![cfg_attr(not(feature = "std"), no_std)]
#![deny(clippy::alloc_instead_of_core)]
#![deny(clippy::std_instead_of_core)]

//! Rust binding for [tcc](https://repo.or.cz/w/tinycc.git)
//!
//! # Example
//! ```
//! use std::ffi::CString;
//!
//! use tcc::{Context, Guard, OutputType};
//! let p = CString::new(
//!     r#"
//!     int add(int a, int b){
//!         return a+b;
//!     }
//!     "#
//!     .as_bytes(),
//! )
//! .unwrap();
//! let mut ctx = Context::new().unwrap();
//! assert!(ctx.compile_string(&p).is_ok());
//! ```

extern crate alloc;

use alloc::{boxed::Box, ffi::CString, string::ToString};
use core::ffi::{CStr, c_char, c_int, c_void};
#[cfg(feature = "std")] use std::path::Path;

use tcc_sys::*;
#[cfg(not(feature = "std"))] use unix_path::Path;

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
/// Output type of the compilation.
pub enum OutputType {
    /// output in memory (default)
    Memory = TCC_OUTPUT_MEMORY,

    /// executable file
    Exe = TCC_OUTPUT_EXE,

    /// dynamic library
    Dll = TCC_OUTPUT_DLL,

    /// object file
    Obj = TCC_OUTPUT_OBJ,

    /// only preprocess (used internally)
    Preprocess = TCC_OUTPUT_PREPROCESS,
}

/// Compilation context.
pub struct Context<'err> {
    inner:    *mut TCCState,
    err_func: Option<Box<Box<dyn 'err + FnMut(&CStr)>>>,
}

/// Real call back of tcc.
extern "C" fn call_back(opaque: *mut c_void, msg: *const c_char) {
    let func: *mut &mut dyn FnMut(&CStr) = opaque as *mut &mut dyn FnMut(&CStr);
    unsafe { (*func)(CStr::from_ptr(msg)) }
}

impl<'err> Context<'err> {
    /// Create a new context builder
    ///
    /// Context can not live together, mutable reference to guard makes compiler
    /// check this. Out of memory is only possible reason of failure.
    pub fn new() -> Result<Self, ()> {
        let inner = unsafe { tcc_new() };
        if inner.is_null() {
            // OOM
            Err(())
        } else {
            Ok(Self {
                inner,
                err_func: None,
            })
        }
    }

    /// set CONFIG_TCCDIR at runtime
    pub fn set_lib_path<T: AsRef<Path>>(&mut self, path: T) -> &mut Self {
        let path = to_cstr(path);
        unsafe {
            tcc_set_lib_path(self.inner, path.as_ptr());
        }
        self
    }

    /// set options as from command line (multiple supported)
    pub fn set_options(&mut self, option: &CStr) -> &mut Self {
        unsafe {
            tcc_set_options(self.inner, option.as_ptr());
        }
        self
    }

    /// set error/warning display callback
    pub fn set_call_back<T>(&mut self, f: T) -> &mut Self
    where
        T: FnMut(&CStr) + 'err,
    {
        let mut user_err_func: Box<Box<dyn FnMut(&CStr)>> = Box::new(Box::new(f));
        // user_err_func.as_mut().
        unsafe {
            tcc_set_error_func(
                self.inner,
                user_err_func.as_mut() as *mut _ as *mut c_void,
                Some(call_back),
            )
        }
        self.err_func = Some(user_err_func);
        self
    }

    /// add include path
    pub fn add_include_path<T: AsRef<Path>>(&mut self, path: T) -> &mut Self {
        let path = to_cstr(path);
        let ret = unsafe { tcc_add_include_path(self.inner, path.as_ptr()) };
        // this api only returns 0.
        assert_eq!(ret, 0);
        self
    }

    /// add in system include path
    pub fn add_sys_include_path<T: AsRef<Path>>(&mut self, path: T) -> &mut Self {
        let path = to_cstr(path);
        let ret = unsafe { tcc_add_sysinclude_path(self.inner, path.as_ptr()) };
        // this api only returns 0.
        assert_eq!(ret, 0);
        self
    }

    /// define preprocessor symbol 'sym'. Can put optional value
    pub fn define_symbol(&mut self, sym: &CStr, val: &CStr) -> *mut Self {
        unsafe {
            tcc_define_symbol(self.inner, sym.as_ptr(), val.as_ptr());
        }
        self
    }

    /// undefine preprocess symbol 'sym'
    pub fn undefine_symbol(&mut self, sym: &CStr) -> &mut Self {
        unsafe { tcc_undefine_symbol(self.inner, sym.as_ptr()) }
        self
    }

    /// output an executable, library or object file. DO NOT call tcc_relocate()
    /// before
    pub fn set_output_type(&mut self, output: OutputType) -> &mut Self {
        let ret = unsafe { tcc_set_output_type(self.inner, output as c_int) };
        assert_eq!(ret, 0);
        self
    }

    /// add a file (C file, dll, object, library, ld script).
    pub fn add_file<T: AsRef<Path>>(&mut self, file: T) -> Result<(), ()> {
        let file = to_cstr(file);
        let ret = unsafe { tcc_add_file(self.inner, file.as_ptr()) };
        map_c_ret(ret)
    }

    ///  compile a string containing a C source.
    pub fn compile_string(&mut self, p: &CStr) -> Result<(), ()> {
        let ret = unsafe { tcc_compile_string(self.inner, p.as_ptr()) };
        map_c_ret(ret)
    }

    /// Equivalent to -Lpath option.
    pub fn add_library_path<T: AsRef<Path>>(&mut self, path: T) -> &mut Self {
        let path = to_cstr(path);
        let ret = unsafe { tcc_add_library_path(self.inner, path.as_ptr()) };
        assert_eq!(ret, 0);
        self
    }

    /// The library name is the same as the argument of the '-l' option.
    pub fn add_library(&mut self, lib_name: &CStr) -> Result<(), ()> {
        let ret = unsafe { tcc_add_library(self.inner, lib_name.as_ptr()) };
        map_c_ret(ret)
    }

    /// Add a symbol to the compiled program.
    ///
    /// # Safety
    /// Symbol need satisfy ABI requirement.
    pub unsafe fn add_symbol(&mut self, sym: &CStr, val: *const c_void) {
        unsafe {
            let ret = tcc_add_symbol(self.inner, sym.as_ptr(), val);
            assert_eq!(ret, 0);
        }
    }

    /// output an executable, library or object file.
    pub fn output_file<T: AsRef<Path>>(&mut self, file_name: T) -> Result<(), ()> {
        let file_name = to_cstr(file_name);
        let ret = unsafe { tcc_output_file(self.inner, file_name.as_ptr()) };

        map_c_ret(ret)
    }

    /// do all relocations (needed before get symbol)
    pub fn relocate<'a>(&'a mut self) -> Result<RelocatedCtx<'a, 'err>, ()> {
        let ret = unsafe { tcc_relocate(self.inner) };
        if ret != 0 {
            return Err(());
        }

        Ok(RelocatedCtx { inner: self })
    }
}

#[cfg(target_family = "unix")]
fn to_cstr<T: AsRef<Path>>(p: T) -> CString {
    use std::os::unix::ffi::OsStrExt;
    CString::new(p.as_ref().as_os_str().as_bytes()).unwrap()
}

#[cfg(target_family = "windows")]
fn to_cstr<T: AsRef<Path>>(p: T) -> CString {
    CString::new(p.as_ref().to_string_lossy().to_string().as_bytes()).unwrap()
}

// preprocessor
impl<'err> Drop for Context<'err> {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe { tcc_delete(self.inner) }
        }
    }
}

fn map_c_ret(code: c_int) -> Result<(), ()> {
    if code == 0 { Ok(()) } else { Err(()) }
}

/// Relocated compilation context
pub struct RelocatedCtx<'a, 'err> {
    inner: &'a mut Context<'err>,
}

impl<'a, 'err> RelocatedCtx<'a, 'err> {
    /// return symbol value or None if not found
    ///
    /// # Safety
    /// Returned addr can not outlive RelocatedCtx itself. It's caller's
    /// responsibility to take care of validity of addr.
    pub unsafe fn get_symbol(&mut self, sym: &CStr) -> Option<*mut c_void> {
        unsafe {
            let addr = tcc_get_symbol(self.inner.inner, sym.as_ptr());
            if addr.is_null() { None } else { Some(addr) }
        }
    }
}

#[cfg(test)] mod tests;
