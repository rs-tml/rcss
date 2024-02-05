use cargo_metadata::MetadataCommand;

use proc_macro2::TokenStream;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
    rc::Rc,
    str::FromStr,
};
use syn::spanned::Spanned;

use macro_visit::Visitor;
// use rcss_core::CssOutput;

type ScopeId = String;
type ModId = Vec<String>;
type Style = String;

#[derive(Clone, Debug)]
enum Order {
    Computed { extend: ModId },
    Calculated { order: u32, root_scope_id: ScopeId },
}

#[derive(Debug, Clone)]
struct LayeredStyles {
    layers: BTreeMap<ScopeId, (u32, String)>,
}

#[derive(Clone)]
struct CollectedStyles {
    struct_css: BTreeMap<ModId, (Order, ScopeId, String)>,
    other_css: Vec<(Order, ScopeId, String)>,
}

#[derive(Debug, Clone)]
struct Styles {
    sorted_styles: BTreeMap<ScopeId, LayeredStyles>,
}
impl Styles {
    fn from_unsorted(styles: CollectedStyles) -> Self {
        let mut unsorted = styles.struct_css;
        let mut sorted_styles = BTreeMap::new();

        // Firstly process all computed, then go to calculated
        let (computed, to_calculate): (Vec<_>, Vec<_>) = unsorted
            .clone()
            .into_iter()
            .partition(|(_, (order, _, _))| matches!(order, Order::Computed { .. }));

        let mut stack = computed
            .into_iter()
            .chain(to_calculate)
            .map(|(file_id, _)| file_id)
            .collect::<Vec<_>>();

        let mut limit_iters = 1000;
        println!("Start sorting styles :{:?}", unsorted);

        while let Some(file_id) = stack.pop() {
            let (order, scope_id, style) = unsorted.get(&file_id).unwrap();
            println!("Loading unsorted: {:?}", file_id);
            println!("Order: {:?}", order);
            println!("Scope id: {:?}", scope_id);
            println!("Style: {:?}", style);

            match order {
                Order::Calculated {
                    order,
                    root_scope_id,
                } => {
                    // insert root
                    if *order == 0 {
                        assert_eq!(root_scope_id, scope_id);
                        assert!(sorted_styles.get(&*root_scope_id).is_none());
                    }
                    let mut layers = BTreeMap::new();
                    layers.insert(scope_id.clone(), (*order, style.clone()));

                    sorted_styles.insert(root_scope_id.clone(), LayeredStyles { layers });
                }
                Order::Computed { extend } => {
                    let (parent_order, _, _) =
                        unsorted.get(extend).expect("Cannot find extended style");
                    let (new_order, root_scope_id) = match parent_order {
                        Order::Calculated {
                            order,
                            root_scope_id,
                        } => {
                            let new_order = order + 1;
                            (new_order, root_scope_id.clone())
                        }
                        _ => {
                            // Return back file to stack
                            stack.push(file_id);
                            // And add item that file extend
                            stack.push(extend.clone());
                            continue;
                        }
                    };
                    let scope_id = scope_id.clone();
                    let style = style.clone();
                    // Update of order should be reflected in map
                    unsorted.insert(
                        file_id.clone(),
                        (
                            Order::Calculated {
                                order: new_order,
                                root_scope_id: root_scope_id.clone(),
                            },
                            scope_id.clone(),
                            style.clone(),
                        ),
                    );
                    // TODO: assert prev is none
                    sorted_styles
                        .get_mut(&root_scope_id)
                        .expect("Layer should exist")
                        .layers
                        .insert(scope_id, (new_order, style.clone()));
                }
            }
            if limit_iters == 0 {
                panic!("Infinite loop detected")
            }
            limit_iters -= 1;
        }
        for (order, scope_id, style) in styles.other_css {
            match order {
                Order::Calculated {
                    order,
                    root_scope_id,
                } => {
                    // insert root
                    if order == 0 {
                        assert_eq!(root_scope_id, scope_id);
                        assert!(sorted_styles.get(&*root_scope_id).is_none());
                    }
                    let mut layers = BTreeMap::new();
                    layers.insert(scope_id.clone(), (order, style.clone()));
                    sorted_styles.insert(root_scope_id.clone(), LayeredStyles { layers });
                }
                Order::Computed { extend } => {
                    let (parent_order, _, _) = unsorted.get(&extend).unwrap();
                    let (new_order, root_scope_id) = match parent_order {
                        Order::Calculated {
                            order,
                            root_scope_id,
                        } => {
                            let new_order = order + 1;
                            (new_order, root_scope_id.clone())
                        }
                        _ => {
                            panic!("Some style extends unresolvable style.")
                        }
                    };
                    let scope_id = scope_id.clone();
                    let style = style.clone();
                    // Update of order should be reflected in map
                    sorted_styles
                        .get_mut(&root_scope_id)
                        .expect("Layer should exist")
                        .layers
                        .insert(scope_id, (new_order, style.clone()));
                }
            }
        }
        Self { sorted_styles }
    }

    fn save_with(&self, config: &BundleOption) {
        let mut resulted_style = String::new();
        for (scope_id, layer) in self.sorted_styles.iter() {
            let mut ordered_layers: Vec<_> = layer
                .layers
                .iter()
                .map(|(scope_id, (order, style))| (scope_id, order, style))
                .collect();
            ordered_layers.sort_by_key(|(_, order, _)| *order);

            let mut header = format!("@layer {}", scope_id);
            for (scope_id, _, _) in &ordered_layers {
                if header.len() > 7 {
                    header.push(',');
                }
                header.push_str(scope_id);
            }
            header.push(';');

            resulted_style.push_str(&header);

            for (scope_id, _, style) in ordered_layers {
                resulted_style.push_str("@layer ");
                resulted_style.push_str(scope_id);
                resulted_style.push_str(" {\n");
                resulted_style.push_str(&style);
                resulted_style.push_str("}\n");
            }
        }
        let output = rcss_core::CssProcessor::process_style(resulted_style.as_str()).unwrap();

        println!("output: {}", config.output_path);
        let file = std::fs::File::create(&config.output_path).expect("Failed to create file");
        let mut writer = std::io::BufWriter::new(file);
        writer
            .write_all(output.style_string().as_bytes())
            .expect("Failed to write to file");
    }
}
impl CollectedStyles {
    fn new() -> Self {
        Self {
            struct_css: BTreeMap::new(),
            other_css: Vec::new(),
        }
    }
    fn compute_scope_id(style: &str) -> ScopeId {
        let id: String = rcss_core::CssProcessor::init_random_class(&style)
            .into_iter()
            .collect();
        id
    }
    fn add_style(&mut self, file_id: Option<ModId>, style: Style, extend: Option<ModId>) {
        println!("Adding style: {:?}", style);
        println!("File id: {:?}", file_id);
        println!("Extend: {:?}", extend);
        let scope_id = Self::compute_scope_id(&style);
        let order = match extend {
            Some(extend) => Order::Computed { extend },
            None => Order::Calculated {
                order: 0,
                root_scope_id: scope_id.clone(),
            },
        };
        if let Some(file_id) = file_id {
            self.struct_css.insert(file_id, (order, scope_id, style));
        } else {
            self.other_css.push((order, scope_id, style));
        }
    }
}

// Returns (StructName, PathToExtend)
fn preprocess(mut style: &str) -> (Option<String>, Option<ModId>) {
    const RCSS: &str = "@rcss";
    let mut struct_name = None;
    let mut extend = None;

    while let Some(pos) = style.find(RCSS) {
        style = &style[(pos + RCSS.len())..];
        style = style.trim_start();
        if style.starts_with('(') {
            let end = style.find(')').expect("Cannot find end of macro call");
            let args = &style[1..end];
            let tts = TokenStream::from_str(args).expect("failed to parse tokenstream");
            let data = rcss_core::rcss_at_rule::RcssAtRuleConfig::from_token_stream(tts)
                .expect("Failed to parse rcss at rule");
            match data {
                rcss_core::rcss_at_rule::RcssAtRuleConfig::Struct(item) => {
                    struct_name = Some(item.ident.to_string());
                }
                rcss_core::rcss_at_rule::RcssAtRuleConfig::Extend(path) => {
                    extend = Some(
                        path.segments
                            .into_iter()
                            .map(|s| s.ident.to_string())
                            .collect(),
                    );
                }
            }
            style = &style[end..];
        }
    }

    (struct_name, extend)
}

// Scan project_path using syn folder, and find all css macro calls.
pub fn process_styles(
    crate_name: &str,
    collect_styles: Rc<RefCell<CollectedStyles>>,
    entrypoint: &Path,
) {
    let rcss_name = std::env::var("CARGO_CRATE_NAME").unwrap_or("rcss".to_string());

    let css_handler = |mod_path: &[String], token_stream: TokenStream| {
        let style = token_stream
            .span()
            .source_text()
            .expect("cannot find source text for macro call");

        let (struct_name, extend) = preprocess(&style);

        let global_struct_id = struct_name.map(|struct_name| {
            let mut root = mod_path.to_vec();
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
                "crate" => {
                    path[0] = crate_name.to_string();
                }
                _ => path.insert(0, crate_name.to_string()),
            }
            path
        });

        collect_styles
            .borrow_mut()
            .add_style(global_struct_id, style, extend);
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

    let mut result = Vec::new();
    for package in metadata.packages {
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
        let mut output_path = std::env::var("OUT_DIR").expect("OUT_DIR_TO_SET");
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

pub fn bundle(root_manifest: &Path) {
    crate::save_root_manifest_path(root_manifest);

    let packange_name = std::env::var("CARGO_PKG_NAME").expect("Expect CARGO_PKG_NAME to be set");

    let (crates, options) =
        get_depend_crate_info_and_options(root_manifest, packange_name.as_str());

    // TODO: Filter deps that not use macro, like (leptos-rcss)

    println!("Found manifests: {:?}", crates);

    let collected_styles = Rc::new(RefCell::new(CollectedStyles::new()));

    for crate_info in crates {
        let entrypoints = get_entrypoints(&crate_info.path_to_manifest);
        for entrypoint in entrypoints {
            println!("Processing entrypoint: {:?}", entrypoint);
            process_styles("", collected_styles.clone(), &entrypoint);
        }
    }
    let styles = Styles::from_unsorted(collected_styles.borrow().clone());
    styles.save_with(&options);
}
