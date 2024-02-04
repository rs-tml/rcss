use std::ops::{Deref, DerefMut};

use crate::{ScopeId, Style};

/// Each scope created with "extend ..." are NewType wrappers
/// Each of this NewTypes are storing information about STYLE and SCOPE_ID in their type.
///
/// But computed fields are stored in root object.
///
pub trait ScopeChain: Into<Self::Root> {
    type Parent;
    type Root;

    /// NOTE: This method should also contain const version of this method.
    /// But because const traits is not yet stabilized, we use const fn in impl of each struct.
    /// But we left this in trait to make it easier to document.
    ///
    /// This method is used to modify existing
    fn _new_root() -> <Self as crate::extend::ScopeChain>::Root {
        todo!()
    }

    /// NOTE: This method should also contain const version of this method.
    /// But because const traits is not yet stabilized, we use const fn in impl of each struct.
    /// But we left this in trait to make it easier to document.
    ///
    /// This method should only wrap root object to type-safe wrapper.
    /// Fields are modified innew_root method.
    fn _from_root(_root: <Self as crate::extend::ScopeChain>::Root) -> Self
    where
        Self: Sized,
    {
        todo!()
    }
}

/// Hack type that is used instead of generic Into<T>.
/// Used to avoid conflicts with type_builder default value for optional generic.
///
/// Example:
/// On component declaration instead of writing:
/// ```
/// css!{@rccs(pub struct Css)}
/// #[component]
/// pub fn my_component<T: Into<Css> + ScopeChainOps>(css: Option<T>) -> impl IntoView {..}
/// ```
/// Which make ergonomics worse since MyComponent is not possible to use without passing `css` prop.
/// One can write:
/// ```
/// css!{@rccs(pub struct Css)}
/// #[component]
/// pub fn my_component(css: Option<StyleChain<Css>>) -> impl IntoView {..}
/// ```
#[derive(Debug, Clone)]
pub struct StyleChain<T> {
    chain: Vec<(ScopeId, Style)>,
    scoped_style: T,
}

impl<T> Default for StyleChain<T>
where
    T: Default,
    Self: From<T>,
{
    fn default() -> Self {
        T::default().into()
    }
}

impl<T> Deref for StyleChain<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.scoped_style
    }
}
impl<T> DerefMut for StyleChain<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.scoped_style
    }
}

pub mod in_chain_ops {
    use super::{ScopeId, Style};
    use std::any::TypeId;
    /// This trait is not a part of public interface, just because we want to support
    /// `StyleChain` type that store its information in runtime.
    pub(super) trait ScopeOpsStatic {
        fn static_get_all_scopes() -> Vec<ScopeId>;
        fn static_root_scope_id() -> ScopeId;

        fn static_for_each(func: impl FnMut(ScopeId, Style));
    }
    impl ScopeOpsStatic for std::convert::Infallible {
        fn static_get_all_scopes() -> Vec<ScopeId> {
            vec![]
        }
        fn static_root_scope_id() -> ScopeId {
            unreachable!()
        }
        fn static_for_each(_func: impl FnMut(ScopeId, Style)) {}
    }
    impl<T> ScopeOpsStatic for T
    where
        T: crate::extend::ScopeChain + crate::ScopeCommon,
        T::Parent: ScopeOpsStatic + 'static,
    {
        fn static_get_all_scopes() -> Vec<ScopeId> {
            let mut scopes = vec![T::SCOPE_ID];
            scopes.extend(T::Parent::static_get_all_scopes());
            scopes
        }
        fn static_root_scope_id() -> ScopeId {
            if TypeId::of::<T::Parent>() == TypeId::of::<std::convert::Infallible>() {
                T::SCOPE_ID
            } else {
                T::Parent::static_root_scope_id()
            }
        }
        fn static_for_each(mut func: impl FnMut(ScopeId, Style)) {
            func(T::SCOPE_ID, T::STYLE);
            T::Parent::static_for_each(func);
        }
    }

    #[allow(private_bounds)]
    pub trait ScopeChainOps: ScopeOpsStatic {
        fn get_all_scopes(&self) -> Vec<ScopeId> {
            Self::static_get_all_scopes()
        }
        fn root_scope_id(&self) -> ScopeId {
            Self::static_root_scope_id()
        }

        fn for_each(&self, func: impl FnMut(ScopeId, Style)) {
            Self::static_for_each(func)
        }
    }
    impl<T> ScopeChainOps for T
    where
        T: ScopeOpsStatic + crate::ScopeCommon + super::ScopeChain,
    {
        fn get_all_scopes(&self) -> Vec<ScopeId> {
            T::static_get_all_scopes()
        }
        fn root_scope_id(&self) -> ScopeId {
            T::static_root_scope_id()
        }

        fn for_each(&self, func: impl FnMut(ScopeId, Style)) {
            T::static_for_each(func)
        }
    }

    impl<T> ScopeOpsStatic for super::StyleChain<T> {
        fn static_get_all_scopes() -> Vec<ScopeId> {
            unreachable!("not part of public interface")
        }
        fn static_root_scope_id() -> ScopeId {
            unreachable!("not part of public interface")
        }
        fn static_for_each(_func: impl FnMut(ScopeId, Style)) {
            unreachable!("not part of public interface")
        }
    }

    impl<T> ScopeChainOps for super::StyleChain<T> {
        fn for_each(&self, mut func: impl FnMut(ScopeId, Style)) {
            self.chain
                .iter()
                .for_each(|(scope, style)| func(*scope, *style));
        }
        fn get_all_scopes(&self) -> Vec<ScopeId> {
            self.chain.iter().map(|(scope, _)| *scope).collect()
        }
        fn root_scope_id(&self) -> ScopeId {
            &self
                .chain
                .last()
                .map(|(scope, _)| *scope)
                .unwrap_or_else(|| unreachable!("No root scope"))
        }
    }
}
impl<T> From<T> for StyleChain<T::Root>
where
    T: in_chain_ops::ScopeOpsStatic + ScopeChain,
    <T as ScopeChain>::Root: From<T>,
{
    fn from(scoped_style: T) -> Self {
        let mut chain = vec![];
        T::static_for_each(|scope, style| chain.push((scope, style)));

        Self {
            chain,
            scoped_style: scoped_style.into(),
        }
    }
}
