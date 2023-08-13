#[cfg(any(feature = "embed-headers", feature = "embed-libraries"))]
macro_rules! visit_base {
    ($length:literal, $($contents:expr)*) => {
        pub static ASSETS: once_cell::sync::Lazy<qp_trie::Trie<qp_trie::wrapper::BString, Asset>> = once_cell::sync::Lazy::new(|| {
            [$($contents,)*].into_iter().collect()
        });
    };
}

#[cfg(any(feature = "embed-headers", feature = "embed-libraries"))]
macro_rules! visit_file {
    (
        $name:literal,
        $id:ident,
        $index:literal,
        $relative_path:literal,
        $absolute_path:literal
    ) => {

        (
            $relative_path.into(),
            Asset({
                include_flate::flate!(pub static DATA: [u8] from $absolute_path);
                &DATA
            }),
        )
    };
}

#[cfg(feature = "embed-headers")]
pub mod headers {
    #[iftree::include_file_tree(
        "
    root_folder_variable = 'OUT_DIR'
    base_folder = 'include/'
    paths = '/**'
    
    [[template]]
    visit_base = 'visit_base'
    visit_file = 'visit_file'
    "
    )]
    #[derive(derive_more::Deref)]
    pub struct Asset(&'static [u8]);
}

#[cfg(feature = "embed-libraries")]
pub mod libraries {
    #[iftree::include_file_tree(
        "
    root_folder_variable = 'CARGO_MANIFEST_DIR'
    base_folder = 'lib/'
    paths = '/**'
    
    [[template]]
    visit_base = 'visit_base'
    visit_file = 'visit_file'
    "
    )]
    #[derive(derive_more::Deref)]
    pub struct Asset(&'static [u8]);
}
