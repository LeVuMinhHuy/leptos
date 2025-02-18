use crate::{HydrationCtx, IntoView};
use cfg_if::cfg_if;
use leptos_reactive::{signal_prelude::*, use_context, RwSignal};
use std::{borrow::Cow, collections::HashMap, error::Error, sync::Arc};

/// A struct to hold all the possible errors that could be provided by child Views
#[derive(Debug, Clone, Default)]
#[repr(transparent)]
pub struct Errors(HashMap<ErrorKey, Arc<dyn Error + Send + Sync>>);

/// A unique key for an error that occurs at a particular location in the user interface.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ErrorKey(Cow<'static, str>);

impl<T> From<T> for ErrorKey
where
    T: Into<Cow<'static, str>>,
{
    #[inline(always)]
    fn from(key: T) -> ErrorKey {
        ErrorKey(key.into())
    }
}

impl IntoIterator for Errors {
    type Item = (ErrorKey, Arc<dyn Error + Send + Sync>);
    type IntoIter = IntoIter;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter())
    }
}

/// An owning iterator over all the errors contained in the [Errors] struct.
#[repr(transparent)]
pub struct IntoIter(
    std::collections::hash_map::IntoIter<
        ErrorKey,
        Arc<dyn Error + Send + Sync>,
    >,
);

impl Iterator for IntoIter {
    type Item = (ErrorKey, Arc<dyn Error + Send + Sync>);

    #[inline(always)]
    fn next(
        &mut self,
    ) -> std::option::Option<<Self as std::iter::Iterator>::Item> {
        self.0.next()
    }
}

/// An iterator over all the errors contained in the [Errors] struct.
#[repr(transparent)]
pub struct Iter<'a>(
    std::collections::hash_map::Iter<
        'a,
        ErrorKey,
        Arc<dyn Error + Send + Sync>,
    >,
);

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a ErrorKey, &'a Arc<dyn Error + Send + Sync>);

    #[inline(always)]
    fn next(
        &mut self,
    ) -> std::option::Option<<Self as std::iter::Iterator>::Item> {
        self.0.next()
    }
}

impl<T, E> IntoView for Result<T, E>
where
    T: IntoView + 'static,
    E: Error + Send + Sync + 'static,
{
    fn into_view(self, cx: leptos_reactive::Scope) -> crate::View {
        let id = ErrorKey(HydrationCtx::peek().previous.into());
        let errors = use_context::<RwSignal<Errors>>(cx);
        match self {
            Ok(stuff) => {
                if let Some(errors) = errors {
                    errors.update(|errors| {
                        errors.0.remove(&id);
                    });
                }
                stuff.into_view(cx)
            }
            Err(error) => {
                match errors {
                    Some(errors) => {
                        errors.update({
                            #[cfg(all(
                                target_arch = "wasm32",
                                feature = "web"
                            ))]
                            let id = id.clone();
                            move |errors: &mut Errors| errors.insert(id, error)
                        });

                        // remove the error from the list if this drops,
                        // i.e., if it's in a DynChild that switches from Err to Ok
                        // Only can run on the client, will panic on the server
                        cfg_if! {
                          if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
                            use leptos_reactive::{on_cleanup, queue_microtask};
                            on_cleanup(cx, move || {
                              queue_microtask(move || {
                                errors.update(|errors: &mut Errors| {
                                  errors.remove(&id);
                                });
                              });
                            });
                          }
                        }
                    }
                    None => {
                        #[cfg(debug_assertions)]
                        warn!(
                            "No ErrorBoundary components found! Returning \
                             errors will not be handled and will silently \
                             disappear"
                        );
                    }
                }
                ().into_view(cx)
            }
        }
    }
}
impl Errors {
    /// Returns `true` if there are no errors.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Add an error to Errors that will be processed by `<ErrorBoundary/>`
    pub fn insert<E>(&mut self, key: ErrorKey, error: E)
    where
        E: Error + Send + Sync + 'static,
    {
        self.0.insert(key, Arc::new(error));
    }

    /// Add an error with the default key for errors outside the reactive system
    pub fn insert_with_default_key<E>(&mut self, error: E)
    where
        E: Error + Send + Sync + 'static,
    {
        self.0.insert(Default::default(), Arc::new(error));
    }

    /// Remove an error to Errors that will be processed by `<ErrorBoundary/>`
    pub fn remove(
        &mut self,
        key: &ErrorKey,
    ) -> Option<Arc<dyn Error + Send + Sync>> {
        self.0.remove(key)
    }

    /// An iterator over all the errors, in arbitrary order.
    #[inline(always)]
    pub fn iter(&self) -> Iter<'_> {
        Iter(self.0.iter())
    }
}
