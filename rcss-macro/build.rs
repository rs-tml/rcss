use std::{
    collections::HashSet,
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

fn main() {
    let out_dir: PathBuf = std::env::var("OUT_DIR")
        .expect("$OUT_DIR should exist.")
        .into();
    let build_dir = out_dir
        .ancestors()
        .skip(2)
        .next()
        .expect("No build directory found.");

    let crates = collect_crates_path(build_dir);

    let warn_on_set = crates.len() > 1;
    if crates.len() > 1 {
        println!("cargo:warning=More than one rcss-bundle root manifest found.");
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
                    if warn_on_set {
                        println!("cargo:warning=Disabling styles from {}", c.display());
                    }
                    config.disable_styles = true;
                }
            }
        }
    }
    config.set_cfg();
}
