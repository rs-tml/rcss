use cargo_metadata::MetadataCommand;

use proc_macro2::TokenStream;
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};
use syn::spanned::Spanned;

use macro_visit::Visitor;
use rcss_core::CssOutput;

mod collect_styles;
pub use collect_styles::*;
// Returns (StructName, PathToExtend)
fn preprocess(style: &str) -> Option<CssOutput> {
    rcss_core::CssProcessor::process_style(style).ok()
}

// Scan project_path using syn folder, and find all css macro calls.
pub fn process_styles(
    crate_name: &str,
    style_collector: Rc<RefCell<collect_styles::Collector>>,
    entrypoint: &Path,
) {
    let rcss_name = std::env::var("CARGO_CRATE_NAME").unwrap_or("rcss".to_string());

    let css_handler = |ctx: macro_visit::MacroContext, token_stream: TokenStream| {
        let style = token_stream
            .span()
            .source_text()
            .expect("cannot find source text for macro call");

        let output = preprocess(&style).expect("Style should be parsable");
        let struct_name = output.declare().map(|s| s.ident.to_string());
        let extend = output.extend().map(|s| {
            s.segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
        });

        let global_struct_id = struct_name.map(|struct_name| {
            let mut root = ctx.mod_path.clone();
            root.push(struct_name);
            root
        });

        // Normalize global_struct_id, make it starts from crate_name
        let global_struct_id = global_struct_id.map(|mut path| {
            if path[0] != crate_name {
                path.insert(0, crate_name.to_string());
            }
            path
        });

        // Normalize import, make it starts from crate_name
        let extend = extend.map(|mut path| {
            match path[0].as_str() {
                "crate" | "" => {
                    path[0] = crate_name.to_string();
                }
                "super" => unimplemented!(
                    "super in rcss extend is not supported (try using global import)"
                ),
                // append mod path to local import
                _ => {
                    path = ctx.mod_path.iter().cloned().chain(path).collect();
                }
            }
            path
        });

        style_collector.borrow_mut().add_style(
            global_struct_id,
            output.class_name().to_string().into(),
            output.style_string().into(),
            extend,
        );
    };
    let mut visitor = Visitor::new();

    let css_struct_paths = vec![format!("{rcss_name}::css")];
    visitor.add_macro(css_struct_paths, css_handler);

    visitor.visit_project(entrypoint);
}

pub fn get_config_from_metadata(
    metadata: &serde_json::Value,
    manifest_root: PathBuf,
) -> Option<BundleOption> {
    println!("Reading metadata: {:?}", metadata);
    let rcss_metadata = metadata.get("rcss")?;

    let mut options = BundleOption::default();
    if let Some(output_path) = rcss_metadata.get("output-path").and_then(|v| v.as_str()) {
        options.output_path = manifest_root.join(output_path).display().to_string();
    }

    if let Some(minify) = rcss_metadata.get("minify").and_then(|v| v.as_bool()) {
        options.minify = minify;
    }
    Some(options)
}
// Read manifest file and find all
// Get list of crates that depend on rcss
// Returns path to their manifest file
pub fn get_depend_crate_info_and_options(
    manifest_path: &Path,
    crate_name: &str,
) -> (Vec<CrateInfo>, BundleOption) {
    let mut cmd = MetadataCommand::new();
    let mut options = BundleOption::default();

    let metadata = cmd
        .manifest_path(manifest_path)
        .exec()
        .expect("Failed to read metadata");

    let blacklist = vec!["rcss-leptos", "rcss-layers"];
    let mut result = Vec::new();
    for package in metadata.packages {
        if blacklist.contains(&package.name.as_str()) {
            continue;
        }
        // Scan metadata of root manifest
        if package.name == crate_name {
            debug_assert_eq!(package.manifest_path, manifest_path);
            let mut path = package.manifest_path.clone();
            path.pop();

            if let Some(new_opts) = get_config_from_metadata(&package.metadata, path.into()) {
                options = new_opts;
            }
        }
        for dep in &package.dependencies {
            if dep.name == "rcss" {
                let crate_info = CrateInfo {
                    name: package.name.clone(),
                    path_to_manifest: package.manifest_path.clone().into(),
                };
                result.push(crate_info);
            }
        }
    }

    return (result, options);
}

// Read manifest file and find all entrypoints.
// Collect them and return as path to src/lib.rs or src/main.rs depending on the project type.
pub fn get_entrypoints(manifest_path: &Path) -> Vec<PathBuf> {
    println!("Reading manifest file: {:?}", manifest_path);
    let manifest = std::fs::read_to_string(manifest_path).unwrap();
    let manifest: toml::Value = toml::from_str(&manifest).unwrap();
    let mut result = Vec::new();
    if let Some(lib) = manifest.get("lib") {
        if let Some(path) = lib.get("path") {
            result.push(path.as_str().unwrap().into());
        }
    }
    if let Some(bin) = manifest.get("bin") {
        for bin in bin.as_array().unwrap() {
            if let Some(path) = bin.get("path") {
                result.push(path.as_str().unwrap().into());
            }
        }
    }

    // Try to check default main.rs and lib.rs entrypoints.
    let default_paths = vec!["src/main.rs", "src/lib.rs"];
    let mut path_to_src: PathBuf = manifest_path.into();
    path_to_src.pop();
    for path in default_paths {
        let path = path_to_src.join(path);
        if path.exists() {
            result.push(path);
        }
    }

    return result;
}

pub enum WatchMode {
    CurrentPackage,
    AllPackages,
}
pub struct BundleOption {
    pub output_path: String,
    pub minify: bool,
    pub watch_mode: WatchMode,
}

impl Default for BundleOption {
    fn default() -> Self {
        let mut output_path = std::env::var("OUT_DIR").expect("OUT_DIR to be set");
        output_path.push_str("/styles.css");
        Self {
            output_path,
            minify: true,
            watch_mode: WatchMode::AllPackages,
        }
    }
}

#[derive(Debug)]
pub struct CrateInfo {
    pub name: String,
    pub path_to_manifest: PathBuf,
}

pub fn bundle(packange_name: String, root_manifest: &Path) {
    crate::save_root_manifest_path(root_manifest);

    let (crates, options) =
        get_depend_crate_info_and_options(root_manifest, packange_name.as_str());

    // TODO: Filter deps that not use macro, like (leptos-rcss)

    println!("Found manifests: {:?}", crates);

    let collected_styles = Rc::new(RefCell::new(collect_styles::Collector::new()));

    for crate_info in crates {
        let entrypoints = get_entrypoints(&crate_info.path_to_manifest);
        for entrypoint in entrypoints {
            println!("Processing entrypoint: {:?}", entrypoint);
            process_styles("", collected_styles.clone(), &entrypoint);
        }
    }
    let styles = collect_styles::Styles::from_unsorted(collected_styles.borrow().clone());
    styles.save_with(&options);
}

pub fn bundle_build_rs() {
    let mut path: PathBuf = std::env::var("CARGO_MANIFEST_DIR").unwrap().into();
    path.push("Cargo.toml");
    let packange_name = std::env::var("CARGO_PKG_NAME").expect("Expect CARGO_PKG_NAME to be set");
    println!("cargo:rerun-if-changed=Cargo.toml");
    bundle(packange_name, &path)
}
