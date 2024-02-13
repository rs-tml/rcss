pub type Style = &'static str;
pub type ScopeId = &'static str;

#[must_use = "Scope style should be registered"]
pub trait ScopeCommon {
    /// Scope that was defined in basic object.
    const SCOPE_ID: &'static str;
    /// Scope style that was defined in basic object.
    /// It should contain valid css style.
    /// Note: It can be empty, if any root crate uses rcss-bundle and sets `metadata.rcss.disable-styles = true`.
    const STYLE: &'static str;

    fn scope_style(&self) -> &'static str {
        Self::STYLE
    }
}

#[cfg(test)]
mod static_test {
    use std::convert::Infallible;

    use crate::{extend::ScopeChain, ScopeCommon};

    impl<'a> std::ops::Index<&'a str> for CssScope {
        type Output = str;
        fn index(&self, index: &'a str) -> &Self::Output {
            match index {
                "foo" => self.foo,
                "bar" => self.bar,
                "baz-2" => self.__kebab__baz_k_2,
                _ => panic!("Has no such key"),
            }
        }
    }

    #[allow(non_snake_case)]
    pub struct CssScope {
        pub foo: &'static str,
        pub bar: &'static str,

        __kebab__baz_k_2: &'static str,
    }

    impl CssScope {
        pub fn new() -> Self {
            Self::new_root()
        }

        /// Const fn is not stabilized in traits, so we use it in structure.
        /// Methods new_root is used to create constant time root object, and modify thier content.
        /// It allow us to have resulted css object on compile time.
        ///
        ///
        /// TODO: Later when const trait will be stabilized we can move it into ScopeChain trait.
        pub const fn new_root() -> Self {
            Self {
                foo: "foo",
                bar: "bar",
                __kebab__baz_k_2: "baz-2",
            }
        }
    }

    impl Default for CssScope {
        fn default() -> Self {
            Self::new()
        }
    }

    impl super::ScopeCommon for CssScope {
        const STYLE: &'static str = ".foo{}.bar{}.baz-2{}";
        const SCOPE_ID: &'static str = "UNIQ_ID";
    }

    impl crate::extend::ScopeChain for CssScope {
        type Parent = Infallible;
        type Root = Self;
    }

    /// Extension;
    pub struct ScopeExtension(CssScope);

    impl ScopeExtension {
        pub fn new() -> Self {
            let root = Self::new_root();
            root.into()
        }

        pub const fn new_root() -> <Self as crate::extend::ScopeChain>::Root {
            const ROOT: <ScopeExtension as ScopeChain>::Root =
                <ScopeExtension as ScopeChain>::Root::new_root();
            let mut root = ROOT;
            root.foo = crate::reexport::const_format::formatcp!("{} {}", ROOT.foo, "foo-3");
            root
        }
    }

    impl std::ops::Deref for ScopeExtension {
        type Target = CssScope;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl std::ops::DerefMut for ScopeExtension {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl Default for ScopeExtension {
        fn default() -> Self {
            Self::new()
        }
    }

    impl super::ScopeCommon for ScopeExtension {
        const STYLE: &'static str = ".foo{}";
        const SCOPE_ID: &'static str = "UNIQ_ID2";
    }

    impl crate::extend::ScopeChain for ScopeExtension {
        type Parent = CssScope;
        type Root = <Self::Parent as crate::extend::ScopeChain>::Root;
    }
    impl From<ScopeExtension> for <ScopeExtension as crate::extend::ScopeChain>::Root {
        fn from(v: ScopeExtension) -> Self {
            v.0.into()
        }
    }

    impl From<<ScopeExtension as crate::extend::ScopeChain>::Root> for ScopeExtension {
        fn from(v: <ScopeExtension as crate::extend::ScopeChain>::Root) -> Self {
            Self(v.into())
        }
    }

    /// Extension;
    pub struct DeepExtension(ScopeExtension);

    impl DeepExtension {
        pub fn new() -> Self {
            let root = Self::new_root();
            root.into()
        }

        pub const fn new_root() -> <Self as ScopeChain>::Root {
            const ROOT: <DeepExtension as ScopeChain>::Root =
                <DeepExtension as ScopeChain>::Root::new_root();
            let mut root = ROOT;
            root.bar = crate::reexport::const_format::formatcp!("{} {}", ROOT.bar, "bar-15");
            root
        }
    }

    impl std::ops::Deref for DeepExtension {
        type Target = ScopeExtension;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl std::ops::DerefMut for DeepExtension {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl Default for DeepExtension {
        fn default() -> Self {
            Self::new()
        }
    }

    impl super::ScopeCommon for DeepExtension {
        const STYLE: &'static str = ".bar{}";
        const SCOPE_ID: &'static str = "UNIQ_ID3";
    }

    impl crate::extend::ScopeChain for DeepExtension {
        type Parent = ScopeExtension;
        type Root = <Self::Parent as crate::extend::ScopeChain>::Root;
    }

    impl From<DeepExtension> for <DeepExtension as crate::extend::ScopeChain>::Root {
        fn from(v: DeepExtension) -> Self {
            v.0.into()
        }
    }

    impl From<<DeepExtension as crate::extend::ScopeChain>::Root> for DeepExtension {
        fn from(v: <DeepExtension as crate::extend::ScopeChain>::Root) -> Self {
            Self(v.into())
        }
    }

    fn assert_scope_common<T: ScopeCommon>(_: &T) {}
    fn assert_scope_chain<T: ScopeChain>(_: &T) {}
    #[test]
    fn find_root_works() {
        use crate::extend::in_chain_ops::ScopeChainOps;
        let deep = DeepExtension::new();
        assert_scope_common(&deep);
        assert_scope_chain(&deep);
        let root = deep.root_scope_id();
        assert_eq!(root, "UNIQ_ID");
        let dyn_style: crate::extend::StyleChain<<DeepExtension as ScopeChain>::Root> = deep.into();
        assert_eq!(dyn_style.root_scope_id(), "UNIQ_ID");
    }

    #[test]
    fn get_all_scopes() {
        use crate::extend::in_chain_ops::ScopeChainOps;
        let deep = DeepExtension::new();
        let scopes = deep.get_all_scopes();
        assert_eq!(scopes, vec!["UNIQ_ID3", "UNIQ_ID2", "UNIQ_ID"]);
        let dyn_style: crate::extend::StyleChain<<DeepExtension as ScopeChain>::Root> = deep.into();
        let scopes = dyn_style.get_all_scopes();
        assert_eq!(scopes, vec!["UNIQ_ID3", "UNIQ_ID2", "UNIQ_ID"]);
    }

    #[test]
    fn for_each_style() {
        use crate::extend::in_chain_ops::ScopeChainOps;
        let deep = DeepExtension::new();
        let mut styles = vec![];
        deep.for_each(|id, style| styles.push((id, style)));
        assert_eq!(
            styles,
            vec![
                ("UNIQ_ID3", ".bar{}"),
                ("UNIQ_ID2", ".foo{}"),
                ("UNIQ_ID", ".foo{}.bar{}.baz-2{}"),
            ]
        );
        let dyn_style: crate::extend::StyleChain<<DeepExtension as ScopeChain>::Root> = deep.into();
        let mut styles = vec![];
        dyn_style.for_each(|id, style| styles.push((id, style)));
        assert_eq!(
            styles,
            vec![
                ("UNIQ_ID3", ".bar{}"),
                ("UNIQ_ID2", ".foo{}"),
                ("UNIQ_ID", ".foo{}.bar{}.baz-2{}"),
            ]
        );
    }
}
