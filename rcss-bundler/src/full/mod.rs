use cargo_metadata::{Metadata, MetadataCommand, Package, PackageId};

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

fn get_crate_id_by_manifest_path(
    metadata: &Metadata,
    manifest_path: &Path,
) -> (PackageId, BundleOption) {
    for package in &metadata.packages {
        if package.manifest_path.as_path() == manifest_path {
            let mut options = BundleOption::default();
            let mut path = package.manifest_path.clone();
            path.pop();

            if let Some(new_opts) = get_config_from_metadata(&package.metadata, path.into()) {
                options = new_opts;
            }
            return (package.id.clone(), options);
        }
    }
    panic!(
        "Failed to find crate id by manifest path: {}",
        manifest_path.display()
    );
}
// For relatively small number of packages (<1000), it's ok to use linear search, even for each dependency.
// But if it will be perfomanse bottleneck, we can firstly build Map<PackageId, Package> to speed up search.
fn get_package_by_id<'a>(metadata: &'a Metadata, package_id: &PackageId) -> &'a Package {
    metadata
        .packages
        .iter()
        .find(|pkg| &pkg.id == package_id)
        .expect(&format!("Failed to find package by id: {}", package_id))
}

// Currently bundler will only support lib and bin targets,
// and dependency can be only lib
fn extract_crate_info(package: &Package, is_lib: bool) -> CrateInfo {
    CrateInfo {
        name: package.name.clone(),
        manifest_path: package.manifest_path.clone().into(),
        entrypoints: package
            .targets
            .iter()
            .filter(|f| {
                if is_lib {
                    f.kind.iter().any(|k| k == "lib")
                } else {
                    f.kind.iter().any(|k| k == "bin" || k == "lib")
                }
            })
            .map(|t| t.src_path.clone().into())
            .collect(),
    }
}
// Read manifest file and find all
// Get list of crates that depend on rcss
// Returns path to their manifest file
pub fn get_depend_crate_info_and_options(manifest_path: &Path) -> (Vec<CrateInfo>, BundleOption) {
    let mut cmd = MetadataCommand::new();

    // panic!("manifest: {}", manifest_path.display());
    let metadata = cmd
        .manifest_path(manifest_path)
        .exec()
        .expect("Failed to read metadata");

    let (root_package, options) = get_crate_id_by_manifest_path(&metadata, manifest_path);
    let blacklist = vec!["rcss-leptos", "rcss-layers"];
    let nodes = &metadata
        .resolve
        .as_ref()
        .expect("Failed to find metadata root resolve graph")
        .nodes;
    let deps_ids = &nodes
        .iter()
        .find(|n| n.id == root_package)
        .expect("Failed to find root package dependencies")
        .dependencies;

    let mut results = vec![extract_crate_info(
        get_package_by_id(&metadata, &root_package),
        false,
    )];

    for dep in deps_ids {
        let package = get_package_by_id(&metadata, &dep);
        if blacklist.contains(&package.name.as_str()) {
            continue;
        }
        let dep_on_rcss = package.dependencies.iter().any(|d| d.name == "rcss");
        if dep_on_rcss {
            results.push(extract_crate_info(&package, true));
        }
    }

    return (results, options);
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
    pub manifest_path: PathBuf,
    pub entrypoints: Vec<PathBuf>,
}

pub fn bundle(root_manifest: &Path) -> String {
    let (crates, options) = get_depend_crate_info_and_options(root_manifest);

    // TODO: Filter deps that not use macro, like (leptos-rcss)

    println!("Found manifests: {:?}", crates);

    let collected_styles = Rc::new(RefCell::new(collect_styles::Collector::new()));

    for crate_info in crates {
        let entrypoints = &crate_info.entrypoints;
        for entrypoint in entrypoints {
            println!("Processing entrypoint: {:?}", entrypoint);
            process_styles(&crate_info.name, collected_styles.clone(), &entrypoint);
        }
    }
    let styles = collect_styles::Styles::from_unsorted(collected_styles.borrow().clone());
    styles.save_with(&options)
}

pub fn bundle_build_rs() {
    let mut path: PathBuf = std::env::var("CARGO_MANIFEST_DIR").unwrap().into();
    path.push("Cargo.toml");
    println!("cargo:rerun-if-changed=Cargo.toml");

    crate::save_root_manifest_path(&path);
    let file_out = bundle(&path);
    println!("cargo:rerun-if-changed={file_out}");
}
