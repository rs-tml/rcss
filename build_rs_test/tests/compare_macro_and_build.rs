use rcss_core::CssOutput;
use regex::Regex;
use std::process::Command;

// Use cargo expand, to generate expanded code.
// compare it with build_helper::collect_styles output.
#[test]
fn test_compare_macro_and_build() {
    let cargo_path = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let cargo_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| "./".to_string());

    let cargo_dir = format!("{}/test_project", cargo_dir);
    let cargo_toml = format!("{cargo_dir}/Cargo.toml");
    dbg!(&cargo_path);
    dbg!(&cargo_dir);
    let expanded = Command::new(&cargo_path)
        .args(&["expand", "--ugly", "--manifest-path", &cargo_toml])
        .output()
        .expect("Failed to run cargo rustc");
    println!("stderr = {}", String::from_utf8(expanded.stderr).unwrap());

    let expanded = String::from_utf8(expanded.stdout).unwrap();

    // Use Regex to find pattern like `class = "**"`` inside fn `main() {}`` of expanded code.
    let re = Regex::new(r#"fn main\(\) \{.*let class = "([^"]+)".*\}"#).unwrap();
    let captures = re.captures(&expanded).unwrap();
    let class_name = captures.get(1).unwrap().as_str();

    println!("test {class_name}");
    // panic!("expanded: {}", expanded);

    let output = rcss::build_helper::process_styles(&cargo_dir, |s| {
        rcss_core::CssProcessor::new(
            rcss_core::CssPreprocessor::LightningCss,
            rcss_core::CssEmbeding::Scoped,
        )
        .process_style(s)
    });
    let output = CssOutput::merge_to_string(&output);
    // panic!();
    assert_eq!(
        output,
        format!(".my-valid.{class_name} {{\n  color: red;\n}}\n")
    )
}
