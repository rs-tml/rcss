use std::ops::Deref;

///
/// Css module with inline style corresponding to the module.
///
pub struct CssWithStyle<C> {
    css: C,
    style: &'static str,
}
impl<C> CssWithStyle<C> {
    /// Create new CssWithStyle instance.
    pub fn new(css: C, style: &'static str) -> Self {
        Self { css, style }
    }

    /// Get inline style string.
    pub fn style(&self) -> &'static str {
        self.style
    }

    /// Destruct object to get fields.
    pub fn destruct(self) -> (C, &'static str) {
        (self.css, self.style)
    }
}

impl<C> Deref for CssWithStyle<C> {
    type Target = C;
    fn deref(&self) -> &Self::Target {
        &self.css
    }
}

pub trait CssCommon {
    /// Style used in basic object
    const BASIC_STYLE: &'static str;
    /// Scope that was defined in basic object.
    const BASIC_SCOPE: &'static str;

    /// Create ancestor basic object.
    fn basic() -> Self
    where
        Self: Sized;

    /// Create basic object with bounded styles.
    fn basic_with_style() -> CssWithStyle<Self>
    where
        Self: Sized,
    {
        CssWithStyle::new(Self::basic(), Self::BASIC_STYLE)
    }
    /// Returns current value of scoped class.
    fn scoped_class(&self) -> &'static str;
}
