use std::{
    collections::HashSet,
    ffi::OsStr,
    path::{Path, PathBuf},
};

///
/// Each root manifest that uses rcss-bundle will have file named "rcss-bundle.config"
///
use rcss_bundle::load_root_manifest_path;

fn collect_crates_path(build_dir: &Path) -> HashSet<PathBuf> {
    let mut crates_paths = HashSet::new();

    for entry in build_dir.read_dir().unwrap() {
        let Ok(dir) = entry else {
            continue;
        };

        let path = dir.path().join("out");

        if !path.is_dir() {
            continue;
        }
        if let Some(path) = load_root_manifest_path(&path) {
            crates_paths.insert(path);
        }
    }

    crates_paths
}

struct Config {
    disable_styles: bool,
}
impl Config {
    fn set_cfg(&self) {
        if self.disable_styles {
            println!("cargo:rustc-cfg=disable_styles");
        }
    }
}

fn traverse_targets_dirs<F>(parent: &Path, mode: &OsStr, mut f: F)
where
    F: FnMut(&Path),
{
    // /target/**
    for entry in parent.read_dir().unwrap() {
        //    /target/wasm32-unknown-unknown/
        // or /target/(mode)/
        // or /target/some_artifact_folder/
        // or /target/.rustc_info.json

        let entry = entry.unwrap();
        let path = entry.path();
        //    /target/wasm32-unknown-unknown/
        // or /target/(mode)/
        // or /target/some_artifact_folder/
        if !path.is_dir() {
            continue;
        }

        // keep only /target/wasm32-unknown-unknown/(mode)/ folders
        let new_path = path.join(mode).join("build");
        if !new_path.is_dir() {
            continue;
        }

        f(&new_path);
    }
}

fn crates_paths_with_subtargets() -> HashSet<PathBuf> {
    // /target/(mode)/build/(package_name)-(hash)/out
    let out_dir: PathBuf = std::env::var("OUT_DIR")
        .expect("$OUT_DIR should exist.")
        .into();
    // /target/(mode)/build/
    let build_dir = out_dir
        .ancestors()
        .skip(2)
        .next()
        .expect("No build directory found.");

    let mut crates = collect_crates_path(build_dir);

    // Macro is always build for native target, therefore build script is also run for native target.
    // But if your end crate is build using cross-compiliation it path can vary.
    // Ex: instead of /target/(mode)/build/ it can be /target/(mode)/(target)/build/
    let mut ancestors = build_dir.ancestors().skip(1);
    let target_mode_dir = ancestors
        .next()
        .expect("No mode found in target dir (release or debug?).");
    let mode = target_mode_dir
        .file_name()
        .expect("Failed to retrieve mod from target dir (release or debug?).");
    let target_dir = ancestors.next().expect("No target dir found.");
    traverse_targets_dirs(target_dir, mode, |path| {
        crates.extend(collect_crates_path(path));
    });
    crates
}

fn main() {
    let crates = crates_paths_with_subtargets();

    let too_many_roots = crates.len() > 1;
    if too_many_roots {
        println!("cargo:warning=More than one rcss-bundle root manifest found.");
        println!(
            "cargo:warning=Checkout 'cargo tree -i --depth 1 rcss-bundle' to see what caused this issue."
        );
        for c in &crates {
            println!("cargo:warning=Root detected at {}", c.display());
        }
        println!("cargo:warning=Using merged config from all root manifests.");
    }

    let mut config = Config {
        disable_styles: false,
    };
    for c in crates {
        println!("cargo:rerun-if-changed={}", c.display());
        let toml_content = std::fs::read_to_string(&c).expect("Failed to read Cargo.toml");
        let toml: toml::Value = toml_content.parse().expect("Failed to parse Cargo.toml");
        if let Some(manifest) = toml
            .get("package")
            .and_then(|p| p.get("metadata"))
            .and_then(|m| m.get("rcss"))
        {
            if let Some(disable_styles) = manifest.get("disable-styles").and_then(|d| d.as_bool()) {
                if disable_styles {
                    if too_many_roots {
                        println!("cargo:warning=Disabling styles from {}", c.display());
                    }
                    config.disable_styles = true;
                }
            }
        }
    }
    config.set_cfg();
}
