use regex::Regex;
use std::{cell::RefCell, path::Path, process::Command, rc::Rc};

// Use cargo expand, to generate expanded code.
// compare it with build_helper::collect_styles output.
#[test]
fn test_compare_macro_and_build() {
    let cargo_path = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let cargo_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| "./".to_string());

    let cargo_dir = format!("{}/test_project", cargo_dir);
    let cargo_toml = format!("{cargo_dir}/Cargo.toml");

    let expanded = Command::new(cargo_path)
        .args(["expand", "--ugly", "--manifest-path", &cargo_toml])
        .output()
        .expect("Failed to run cargo rustc");
    println!("stderr = {}", String::from_utf8(expanded.stderr).unwrap());

    let expanded = String::from_utf8(expanded.stdout).unwrap();

    // Use Regex to find pattern like `my_valid: "**"`` inside fn `main() {}`` of expanded code.
    // It will be class name initialization.
    let re =
        Regex::new(r#"fn main\(\) \{(.|\r\n|\r|\n)+my_valid: "([^"]+)"(.|\r\n|\r|\n)+const STYLE: &'static str = "([^"]+)"(.|\r\n|\r|\n)+\}"#).unwrap();
    let captures = re.captures(&expanded).unwrap();
    let class_name = captures.get(2).unwrap().as_str();
    let captures = re.captures(&expanded).unwrap();
    let style = captures.get(4).unwrap();
    println!("test {class_name}");
    // panic!("expanded: {}", expanded);

    let style_collector = Rc::new(RefCell::new(rcss_bundler::full::Collector::new()));
    let cargo_dir: &Path = cargo_dir.as_ref();
    let cargo_dir = cargo_dir.join("src/valid.rs");
    rcss_bundler::process_styles("test_project", style_collector.clone(), &cargo_dir);
    let output = style_collector.borrow().to_styles();
    let output = &output[0];
    // panic!();
    assert_eq!(output, &format!(".{class_name}{{color:red}}"));
    assert_eq!(output, style.as_str())
}
