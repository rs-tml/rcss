#[cfg(test)]
mod test {
    use std::{cell::RefCell, path::Path, rc::Rc};

    #[test]
    fn test_collect_modules_with_lightning() {
        let style_collector = Rc::new(RefCell::new(rcss_bundle::full::Collector::new()));
        let cargo_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| "./".to_string());
        let cargo_dir: &Path = cargo_dir.as_ref();
        let cargo_dir = cargo_dir.join("test_files/src/file.rs");

        rcss_bundle::process_styles("test_files", style_collector.clone(), cargo_dir.as_ref());
        let output = style_collector.borrow().to_styles();
        let output = output.join("");

        assert_eq!(output, ".my-class2-XUSD{color:#00f}.my-class-Mlfe{color:red}.container-PTCU{background-color:#000}")
    }
}
