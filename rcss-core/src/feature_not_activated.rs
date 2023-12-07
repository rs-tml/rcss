use crate::{CssStyleProcessor, FragmentVisitor, SelectorFragment};

pub struct Preprocessor;

pub struct NoSelector;
impl SelectorFragment for NoSelector {
    fn append_new_class(&mut self, _class: &str) {}
}

impl CssStyleProcessor<'_> for Preprocessor {
    type Fragment = NoSelector;

    fn load_style(_style: &str) -> Self {
        panic!(
            "Feature for {module} is not activated",
            module = module_path!()
        )
    }

    fn visit_modify<F>(&mut self, _visitor: F)
    where
        F: FragmentVisitor<Fragment = Self::Fragment>,
    {
        panic!(
            "Feature for {module} is not activated",
            module = module_path!()
        )
    }
    fn to_string(&self) -> String {
        panic!(
            "Feature for {module} is not activated",
            module = module_path!()
        )
    }
}
