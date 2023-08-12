use std::{env, fs, path::PathBuf};

use cfg_if::cfg_if;
use static_assertions::const_assert;

const ARCH: &[()] = &[
    #[cfg(feature = "arch-i386")]
    (),
    #[cfg(feature = "arch-arm32")]
    (),
    #[cfg(feature = "arch-arm64")]
    (),
    #[cfg(feature = "arch-c67")]
    (),
    #[cfg(feature = "arch-x86_64")]
    (),
    #[cfg(feature = "arch-rv64")]
    (),
];

// Make sure that either 0 or 1 arch is selected
const_assert!(ARCH.len() < 1);

const LINK: &[()] = &[
    #[cfg(feature = "link-pe")]
    (),
    #[cfg(feature = "link-mach-o")]
    (),
];

fn generate_bindings() -> eyre::Result<()> {
    let bindings = bindgen::Builder::default()
        .header("tinycc/libtcc.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .use_core()
        .generate()?;
    let out_path = PathBuf::from(env::var("OUT_DIR")?);
    bindings.write_to_file(out_path.join("bindings.rs"))?;
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    bindings.write_to_file(manifest_dir.join("bindings.rs"))?;
    Ok(())
}

fn build_static_library() -> eyre::Result<()> {
    let mut cc = cc::Build::new();
    let dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let version = format!(
        r#""{}""#,
        fs::read_to_string(dir.join("tinycc").join("VERSION"))?.trim_end()
    );

    let cc = cc
        .file("tinycc/libtcc.c")
        .include(&dir)
        .define("TCC_VERSION", version.as_str());

    let mut defines = vec![];

    let target: Option<&'static str> = if ARCH.len() == 1 {
        cfg_if! {
            if #[cfg(feature = "arch-i386")] {
                Some("TCC_TARGET_I386")
            } else if #[cfg(feature = "arch-arm32")] {
                Some("TCC_TARGET_ARM")
            } else if #[cfg(feature = "arch-arm64")] {
                Some("TCC_TARGET_ARM64")
            } else if #[cfg(feature = "arch-c67")] {
                Some("TCC_TARGET_C67")
            }else if #[cfg(feature = "arch-x86_64")] {
                Some("TCC_TARGET_X86_64")
            } else if #[cfg(feature = "arch-rv64")] {
                Some("TCC_TARGET_RISCV64")
            } else {
                None
            }
        }
    } else {
        cfg_if! {
            if #[cfg(target_arch = "x86")] {
                Some("TCC_TARGET_I386")
            } else if #[cfg(target_arch = "arm")] {
                Some("TCC_TARGET_ARM")
            } else if #[cfg(target_arch = "aarch64")] {
                Some("TCC_TARGET_ARM64")
            } else if #[cfg(target_arch = "x86_64")] {
                Some("TCC_TARGET_X86_64")
            } else if #[cfg(target_arch = "riscv64")] {
                Some("TCC_TARGET_RISCV64")
            } else {
                None
            }
        }
    };

    defines.push(target.unwrap());

    if LINK.len() == 0 {
        cfg_if! {
            if #[cfg(target_os = "windows")] {
                defines.push("TCC_TARGET_PE")
            } else if #[cfg(target_os = "macos")] {
                defines.push("TCC_TARGET_MACHO")
            } else {
            }
        }
    } else {
        if cfg!(feature = "link-pe") {
            defines.push("TCC_TARGET_PE")
        }
        if cfg!(feature = "link-mach-o") {
            defines.push("TCC_TARGET_MACHO")
        }
    }

    for def in defines {
        cc.define(def, None);
    }

    cc.try_compile("libtcc")?;
    Ok(())
}

fn link_dynamic_library() -> eyre::Result<()> {
    todo!()
}

fn main() -> eyre::Result<()> {
    generate_bindings()?;

    if cfg!(feature = "vendored") {
        build_static_library()?;
    } else {
        link_dynamic_library()?;
    }

    Ok(())
}
