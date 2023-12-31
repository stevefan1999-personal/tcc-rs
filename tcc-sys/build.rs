use std::{env, fs, path::PathBuf};

use cargo_emit::rerun_if_changed;
use cfg_if::cfg_if;
use eyre::Result;
use static_assertions::const_assert;
use strum::IntoStaticStr;

#[derive(IntoStaticStr, Copy, Clone)]
#[allow(dead_code)]
enum SupportedArchitecture {
    #[strum(serialize = "TCC_TARGET_I386")]
    I386,
    #[strum(serialize = "TCC_TARGET_ARM")]
    ARM32,
    #[strum(serialize = "TCC_TARGET_ARM64")]
    ARM64,
    #[strum(serialize = "TCC_TARGET_C67")]
    C67,
    #[strum(serialize = "TCC_TARGET_X86_64")]
    X86_64,
    #[strum(serialize = "TCC_TARGET_RISCV64")]
    RV64,
}

#[derive(IntoStaticStr, Copy, Clone)]
#[allow(dead_code)]
enum ExecutableLinkage {
    #[strum(serialize = "TCC_TARGET_PE")]
    PortableExecutable,
    #[strum(serialize = "TCC_TARGET_MACHO")]
    MachO,
    ELF,
}

const ARCH: &[SupportedArchitecture] = &[
    #[cfg(feature = "arch-i386")]
    SupportedArchitecture::I386,
    #[cfg(feature = "arch-arm32")]
    SupportedArchitecture::ARM32,
    #[cfg(feature = "arch-arm64")]
    SupportedArchitecture::ARM64,
    #[cfg(feature = "arch-c67")]
    SupportedArchitecture::C67,
    #[cfg(feature = "arch-x86_64")]
    SupportedArchitecture::X86_64,
    #[cfg(feature = "arch-rv64")]
    SupportedArchitecture::RV64,
];

// Make sure that either 0 or 1 arch is selected
const_assert!(ARCH.len() <= 1);

const LINK: &[()] = &[
    #[cfg(feature = "link-pe")]
    ExecutableLinkage::PortableExecutable,
    #[cfg(feature = "link-mach-o")]
    ExecutableLinkage::MachO,
];

// Make sure that either 0 or 1 link is selected
const_assert!(LINK.len() <= 1);

fn generate_bindings() -> Result<()> {
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

fn build_static_library() -> Result<()> {
    let mut cc = cc::Build::new();
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let version = format!(
        r#""{}""#,
        fs::read_to_string(manifest_dir.join("tinycc").join("VERSION"))?.trim_end()
    );

    let cc = cc
        .file("tinycc/libtcc.c")
        .include(&manifest_dir)
        .define("TCC_VERSION", version.as_str());

    let mut defines: Vec<&'static str> = vec![];

    let target: Option<SupportedArchitecture> = if ARCH.len() == 1 {
        cfg_if! {
            if #[cfg(feature = "arch-i386")] {
                Some(SupportedArchitecture::I386)
            } else if #[cfg(feature = "arch-arm32")] {
                Some(SupportedArchitecture::ARM32)
            } else if #[cfg(feature = "arch-arm64")] {
                Some(SupportedArchitecture::ARM64)
            } else if #[cfg(feature = "arch-c67")] {
                Some(SupportedArchitecture::C67)
            }else if #[cfg(feature = "arch-x86_64")] {
                Some(SupportedArchitecture::X86_64)
            } else if #[cfg(feature = "arch-rv64")] {
                Some(SupportedArchitecture::RV64)
            } else {
                panic!("must select a valid target")
            }
        }
    } else {
        cfg_if! {
            if #[cfg(target_arch = "x86")] {
                Some(SupportedArchitecture::I386)
            } else if #[cfg(target_arch = "arm")] {
                Some(SupportedArchitecture::ARM32)
            } else if #[cfg(target_arch = "aarch64")] {
                Some(SupportedArchitecture::ARM64)
            } else if #[cfg(target_arch = "x86_64")] {
                Some(SupportedArchitecture::X86_64)
            } else if #[cfg(target_arch = "riscv64")] {
                Some(SupportedArchitecture::RV64)
            } else {
                panic!("this target is not natively supported")
            }
        }
    };

    defines.push(target.unwrap().into());

    let linkage = if LINK.len() == 0 {
        cfg_if! {
            if #[cfg(target_os = "windows")] {
                Some(ExecutableLinkage::PortableExecutable)
            } else if #[cfg(target_os = "macos")] {
                Some(ExecutableLinkage::MachO)
            } else {
                None
            }
        }
    } else {
        if cfg!(feature = "link-pe") {
            Some(ExecutableLinkage::PortableExecutable)
        } else if cfg!(feature = "link-mach-o") {
            Some(ExecutableLinkage::MachO)
        } else {
            None
        }
    };

    if let Some(target) = target {
        cc.define(target.into(), None);
    }

    if let Some(linkage) = linkage {
        cc.define(linkage.into(), None);
    }

    if cfg!(feature = "vfs") {
        cc.define("CONFIG_VFS", None);
        cc.define("open", "vfs_open");
        cc.define("read", "vfs_read");
        cc.define("lseek", "vfs_lseek");
        cc.define("close", "vfs_close");
    }

    cc.try_compile("libtcc")?;
    Ok(())
}

fn link_dynamic_library() -> Result<()> {
    todo!()
}

#[cfg(feature = "embed-headers")]
fn generate_include_dir() -> Result<()> {
    use eyre::eyre;
    use fs_extra::dir::CopyOptions;

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    let dir_copy_opt = CopyOptions::new()
        .overwrite(true)
        .skip_exist(true)
        .copy_inside(true)
        .content_only(true);

    let include_dir = out_dir.join("include");
    let _ = fs::remove_dir_all(&include_dir);

    fs_extra::dir::copy(
        &manifest_dir.join("tinycc").join("include"),
        &include_dir.join("base"),
        &dir_copy_opt,
    )
    .map_err(|e| eyre!(e))?;

    if cfg!(feature = "embed-headers-win32") || cfg!(target_os = "windows") {
        fs_extra::dir::copy(
            &manifest_dir.join("tinycc").join("win32").join("include"),
            &include_dir.join("win32"),
            &dir_copy_opt,
        )
        .map_err(|e| eyre!(e))?;
    }

    Ok(())
}

fn main() -> Result<()> {
    rerun_if_changed!("tinycc");
    rerun_if_changed!("config.h");
    rerun_if_changed!("build.rs");
    generate_bindings()?;

    if cfg!(feature = "vendored") {
        build_static_library()?;
    } else {
        link_dynamic_library()?;
    }

    #[cfg(feature = "embed-headers")]
    {
        generate_include_dir()?;
    }

    Ok(())
}
