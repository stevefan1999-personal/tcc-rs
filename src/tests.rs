use std::{
    env::temp_dir,
    ffi::{c_int, CString},
    fs::{remove_file, write},
    intrinsics::transmute,
};

use crate::{Context, Guard, OutputType};

#[test]
fn guard_multiple_creat() {
    {
        let g1 = Guard::new();
        assert!(g1.is_ok());
        let g2 = Guard::new();
        assert!(g2.is_err());
    }
    let g3 = Guard::new();
    assert!(g3.is_ok());
}

#[test]
fn set_call_back() {
    let err_p = CString::new("error".as_bytes()).unwrap();
    let mut call_back_ret = None;
    let mut g = Guard::new().unwrap();
    let mut ctx = Context::new(&mut g).unwrap();
    ctx.set_output_type(OutputType::Memory);
    ctx.set_call_back(|_| call_back_ret = Some("called"));
    assert!(ctx.compile_string(&err_p).is_err());
    drop(ctx);
    assert_eq!(call_back_ret, Some("called"));
}

#[test]
fn add_sys_include_path() {
    let p = CString::new("#include<libtcc_test_0_9_27.h>").unwrap();
    let header = "#define TEST";
    let dir = temp_dir();
    write(dir.join("libtcc_test_0_9_27.h"), header).unwrap();

    let mut g = Guard::new().unwrap();
    let mut ctx = Context::new(&mut g).unwrap();
    ctx.set_output_type(OutputType::Memory);
    assert!(ctx.add_sys_include_path(&dir).compile_string(&p).is_ok());
    remove_file(dir.join("libtcc_test_0_9_27.h")).unwrap();
}

#[test]
fn add_include_path() {
    let p = CString::new("#include\"libtcc_test_0_9_27.h\"").unwrap();
    let header = "#define TEST";
    let dir = temp_dir();
    write(dir.join("libtcc_test_0_9_27.h"), header).unwrap();

    let mut g = Guard::new().unwrap();
    let mut ctx = Context::new(&mut g).unwrap();

    ctx.set_output_type(OutputType::Memory);
    assert!(ctx.add_include_path(&dir).compile_string(&p).is_ok());
    remove_file(dir.join("libtcc_test_0_9_27.h")).unwrap();
}

#[test]
fn symbol_define() {
    let p = CString::new(
        r#"#ifdef TEST
        typedef __unknown_type a1;
        #endif
        "#
        .as_bytes(),
    )
    .unwrap();
    let sym = CString::new("TEST".as_bytes()).unwrap();
    let val = CString::new("1".as_bytes()).unwrap();
    let mut g = Guard::new().unwrap();
    let mut ctx = Context::new(&mut g).unwrap();
    ctx.set_output_type(OutputType::Memory);
    ctx.define_symbol(&sym, &val);
    assert!(ctx.compile_string(&p).is_err());
    ctx.undefine_symbol(&sym);
    assert!(ctx.compile_string(&p).is_ok());
}

#[test]
fn output_exe_file() {
    let p = CString::new(
        r#"
        #include<stdio.h>
        int main(int argc, char **argv){
            printf("hello world");
            return 0;
        }
        "#
        .as_bytes(),
    )
    .unwrap();

    let mut g = Guard::new().unwrap();
    let mut ctx = Context::new(&mut g).unwrap();
    ctx.set_output_type(OutputType::Exe);
    assert!(ctx.compile_string(&p).is_ok());
    let dir = temp_dir();
    let exe = dir.join("a.out");
    ctx.output_file(&exe).unwrap();
    assert!(exe.exists());
    remove_file(&exe).unwrap();
}

#[test]
fn output_lib() {
    let p = CString::new(
        r#"
        int add(int a, int b){
            return a+b;
        }
        "#
        .as_bytes(),
    )
    .unwrap();

    let mut g = Guard::new().unwrap();
    let mut ctx = Context::new(&mut g).unwrap();
    ctx.set_output_type(OutputType::Dll);
    assert!(ctx.compile_string(&p).is_ok());
    let dir = temp_dir();
    let lib = dir.join("lib");
    ctx.output_file(&lib).unwrap();
    assert!(lib.exists());
    remove_file(&lib).unwrap();
}

#[test]
fn output_obj() {
    let p = CString::new(
        r#"
        int add(int a, int b){
            return a+b;
        }
        "#
        .as_bytes(),
    )
    .unwrap();

    let mut g = Guard::new().unwrap();
    let mut ctx = Context::new(&mut g).unwrap();
    ctx.set_output_type(OutputType::Obj);
    assert!(ctx.compile_string(&p).is_ok());
    let dir = temp_dir();
    let obj = dir.join("obj");

    ctx.output_file(&obj).unwrap();
    assert!(obj.exists());
    remove_file(&obj).unwrap();
}

#[test]
fn run_func() {
    let p = CString::new(
        r#"
        int add(int a, int b){
            return a+b;
        }
        "#
        .as_bytes(),
    )
    .unwrap();
    let sym = CString::new("add".as_bytes()).unwrap();

    let mut g = Guard::new().unwrap();
    let mut ctx = Context::new(&mut g).unwrap();
    ctx.set_output_type(OutputType::Memory);
    assert!(ctx.compile_string(&p).is_ok());
    let mut relocated = ctx.relocate().unwrap();

    let add: fn(c_int, c_int) -> c_int = unsafe { transmute(relocated.get_symbol(&sym).unwrap()) };
    assert_eq!(add(1, 1), 2);
}

#[test]
fn add_symbol() {
    let p = CString::new(
        r#"
        int add(int a, int b){
            return a+b;
        }
        "#
        .as_bytes(),
    )
    .unwrap();
    let sym = CString::new("add".as_bytes()).unwrap();
    let p2 = CString::new(
        r#"
        int add(int a, int b);
        int add2(int a, int b){
            return add(a, b) + add(a, b);
        }
        "#
        .as_bytes(),
    )
    .unwrap();
    let sym2 = CString::new("add2".as_bytes()).unwrap();

    let mut g = Guard::new().unwrap();
    let mut ctx = Context::new(&mut g).unwrap();
    ctx.set_output_type(OutputType::Memory);
    assert!(ctx.compile_string(&p).is_ok());
    let mut relocated = ctx.relocate().unwrap();
    let add = unsafe { relocated.get_symbol(&sym).unwrap() };

    let mut ctx2 = Context::new(&mut g).unwrap();
    ctx2.set_output_type(OutputType::Memory);
    assert!(ctx2.compile_string(&p2).is_ok());
    unsafe {
        ctx2.add_symbol(&sym, add);
    }
    let mut relocated = ctx2.relocate().unwrap();
    let add2: fn(c_int, c_int) -> c_int =
        unsafe { transmute(relocated.get_symbol(&sym2).unwrap()) };

    assert_eq!(add2(1, 1), 4);
}

#[test]
fn link_lib() {
    let dir = temp_dir();
    let lib = dir.join("libadd.a");

    let p = CString::new(
        r#"
        int __cdecl add(int a, int b){
            return a+b;
        }
        "#
        .as_bytes(),
    )
    .unwrap();

    let mut g = Guard::new().unwrap();
    let mut ctx = Context::new(&mut g).unwrap();
    ctx.set_output_type(OutputType::Dll);
    assert!(ctx.compile_string(&p).is_ok());

    ctx.output_file(&lib).unwrap();
    assert!(lib.exists());

    let p2 = CString::new(
        r#"
        int __cdecl add(int a, int b);
        int __cdecl add2(int a, int b){
            return add(a, b) + add(a, b);
        }
        "#
        .as_bytes(),
    )
    .unwrap();
    let lib_name = CString::new("add".as_bytes()).unwrap();
    let sym2 = CString::new("add2".as_bytes()).unwrap();
    let mut ctx2 = Context::new(&mut g).unwrap();
    ctx2.set_output_type(OutputType::Memory)
        .add_library_path(&dir)
        .add_library(&lib_name)
        .unwrap();

    assert!(ctx2.compile_string(&p2).is_ok());
    let relocate = ctx2.relocate();
    let mut r = relocate.unwrap();

    let add2: fn(c_int, c_int) -> c_int = unsafe { transmute(r.get_symbol(&sym2).unwrap()) };

    assert_eq!(add2(1, 1), 4);
    remove_file(lib).unwrap();
}
