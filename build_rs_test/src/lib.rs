#[cfg(test)]
mod test {
    use rcss_core::CssOutput;

    #[test]
    fn test_collect_modules_with_lightning() {
        let output = rcss::build_helper::process_styles("./test_files", |s| {
            rcss_core::CssProcessor::new(
                rcss_core::CssPreprocessor::LightningCss,
                rcss_core::CssEmbeding::CssModules,
            )
            .process_style(s)
        });

        let output = CssOutput::merge_to_string(&output);
        assert_eq!(output, ".my-class2-kFmk {\n  color: #00f;\n}\n.my-class-GrC5 {\n  color: red;\n}\n.container-zGGy {\n  background-color: #000;\n}\n")
    }

    #[test]
    fn test_collect_scoped_with_lightning() {
        let output = rcss::build_helper::process_styles("./test_files", |s| {
            rcss_core::CssProcessor::new(
                rcss_core::CssPreprocessor::LightningCss,
                rcss_core::CssEmbeding::Scoped,
            )
            .process_style(s)
        });

        let output = CssOutput::merge_to_string(&output);
        assert_eq!(output, ".my-class2._kFmkd8 {\n  color: #00f;\n}\n.my-class._GrC5Fp {\n  color: red;\n}\n.container._zGGyFA {\n  background-color: #000;\n}\n")
    }
    #[test]
    fn test_collect_scoped_with_stylers() {
        let output = rcss::build_helper::process_styles("./test_files", |s| {
            rcss_core::CssProcessor::new(
                rcss_core::CssPreprocessor::StylersCore,
                rcss_core::CssEmbeding::Scoped,
            )
            .process_style(s)
        });

        let output = CssOutput::merge_to_string(&output);
        assert_eq!(output, ".my-class2._kFmkd8{ color: blue; }.my-class._GrC5Fp{ color: red; }.container._zGGyFA{ background-color: black; }")
    }
}
