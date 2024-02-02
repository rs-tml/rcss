#[cfg(test)]
mod test {
    use rcss_core::CssOutput;

    #[test]
    fn test_collect_modules_with_lightning() {
        let output = rcss::build_helper::process_styles("./test_files", |s| {
            rcss_core::CssProcessor::process_style(s).unwrap()
        });

        let output = CssOutput::merge_to_string(&output);
        assert_eq!(output, ".my-class2-kFmk{color:#00f}.my-class-GrC5{color:red}.container-zGGy{background-color:#000}")
    }

    #[test]
    fn test_collect_scoped_with_lightning() {
        let output = rcss::build_helper::process_styles("./test_files", |s| {
            rcss_core::CssProcessor::process_style(s).unwrap()
        });

        let output = CssOutput::merge_to_string(&output);
        assert_eq!(output, ".my-class2._kFmkd8{color:#00f}.my-class._GrC5Fp{color:red}.container._zGGyFA{background-color:#000}")
    }
}
