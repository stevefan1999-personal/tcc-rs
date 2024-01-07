use std::{
    cell::RefCell,
    ffi::{CStr, CString},
    mem::transmute,
    rc::Rc,
};

use tcc::*;

static GREET: &str = r#"
#include <stdio.h>
void greet() {
    printf("hello, rust\n");
}
"#;

fn main() {
    let c_program = CString::new(GREET.as_bytes()).unwrap();
    let ctxs = tcc::scoped(|scope| {
        let err_warn: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let ctx = scope.spawn().unwrap();
        ctx.add_sys_include_path("/usr/include/x86_64-linux-gnu")
            .add_library_path("/usr/lib/x86_64-linux-gnu")
            .add_sys_include_path("/vfs/headers/base")
            .add_sys_include_path("/vfs/headers/win32")
            .add_library_path("/vfs/libraries");

        let compile_ret = ctx
            .set_output_type(OutputType::Memory)
            .set_call_back({
                let err_warn = err_warn.clone();
                move |msg| {
                    *err_warn.borrow_mut() = Some(String::from(msg.to_str().unwrap()));
                    eprintln!("{:?}", err_warn.borrow());
                }
            })
            .compile_string(&c_program);
        if compile_ret.is_err() {
            eprintln!("{:?}", err_warn.borrow());
            Err(())
        } else {
            let mut relocated = ctx.relocate().unwrap();
            let addr = unsafe {
                relocated
                    .get_symbol(CStr::from_bytes_with_nul_unchecked("greet\0".as_bytes()))
                    .unwrap()
            };
            let greet: fn() = unsafe { transmute(addr) };
            greet();
            Ok(greet)
        }
    })
    .unwrap();
    let func = ctxs.get();
    println!("before func");
    func.unwrap()();
    println!("after func");
}
