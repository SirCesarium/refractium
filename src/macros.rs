// @swt-disable max-repetition

/// Macro to define a new protocol by implementing the `RefractiumProtocol` trait.
///
/// This macro simplifies the creation of simple protocols that identify themselves
/// by checking if the initial data slice matches a specific condition.
///
/// # Example
///
/// ```rust
/// use refractium::{define_protocol, types::Transport};
///
/// define_protocol!(
///     /// My custom protocol identifier.
///     name: MyProto,
///     transport: Transport::Tcp,
///     identify: |data| data.starts_with(b"MY_MAGIC")
/// );
/// ```
#[macro_export]
macro_rules! define_protocol {
    (
        $(#[$meta:meta])*
        name: $name:ident,
        transport: $transport:expr,
        identify: |$data:ident| $body:expr
    ) => {
        $(#[$meta])*
        #[derive(Clone)]
        pub struct $name;

        impl $crate::core::types::RefractiumProtocol for $name {
            #[inline]
            fn identify(self: std::sync::Arc<Self>, $data: &[u8]) -> Option<$crate::core::types::ProtocolMatch> {
                use $crate::heck::ToSnakeCase;
                if $body {
                    return Some($crate::core::types::ProtocolMatch {
                        name: stringify!($name).to_snake_case(),
                        metadata: None,
                        implementation: self,
                    });
                }
                None
            }

            fn name(&self) -> String {
                use $crate::heck::ToSnakeCase;
                stringify!($name).to_snake_case()
            }

            fn transport(&self) -> $crate::core::types::Transport {
                $transport
            }
        }
    };
}

/// Macro to quickly define a new protocol hook using a closure.
///
/// # Example
///
/// ```rust
/// use refractium::define_hook;
///
/// define_hook!(MyHook, |ctx, dir, pkt| {
///     println!("Captured {} bytes", pkt.len());
/// });
/// ```
#[cfg(feature = "hooks")]
#[macro_export]
macro_rules! define_hook {
    ($name:ident, |$ctx:ident, $dir:ident, $pkt:ident| $body:expr) => {
        #[derive(Clone)]
        pub struct $name;
        impl $crate::protocols::hooks::ProtocolHook for $name {
            fn name(&self) -> &'static str {
                stringify!($name)
            }
            fn on_packet(
                &self,
                $ctx: &$crate::protocols::hooks::HookContext,
                $dir: $crate::protocols::hooks::Direction,
                $pkt: $crate::bytes::Bytes,
            ) {
                $body
            }
        }
    };
}

/// Wraps an existing protocol with one or more hooks.
///
/// This macro generates a wrapper that implements [`RefractiumProtocol`] and
/// automatically attaches the provided hooks to the connection upon identification.
///
/// # Example
///
/// ```rust
/// use refractium::{hook_protocol, Http, define_hook};
///
/// define_hook!(Logger, |ctx, dir, pkt| { /* ... */ });
///
/// hook_protocol!(
///     wrapper: HookedHttp,
///     proto: Http,
///     hooks: [Logger]
/// );
/// ```
#[cfg(feature = "hooks")]
#[macro_export]
macro_rules! hook_protocol {
    (
        wrapper: $wrapper:ident,
        proto: $proto:ident,
        hooks: [$($hook:expr),* $(,)?]
    ) => {
        #[derive(Clone)]
        pub struct $wrapper {
            inner: $proto,
            hooks: Vec<std::sync::Arc<dyn $crate::protocols::hooks::ProtocolHook>>,
        }

        impl $wrapper {
            /// Creates a new instance with the default set of hooks.
            pub fn new() -> Self {
                Self {
                    inner: $proto,
                    hooks: vec![$(std::sync::Arc::new($hook)),*],
                }
            }

            /// Creates a new instance with a custom set of hooks.
            pub fn with_hooks(hooks: Vec<std::sync::Arc<dyn $crate::protocols::hooks::ProtocolHook>>) -> Self {
                Self {
                    inner: $proto,
                    hooks,
                }
            }
        }

        impl $crate::core::types::RefractiumProtocol for $wrapper {
            #[inline]
            fn identify(self: std::sync::Arc<Self>, data: &[u8]) -> Option<$crate::core::types::ProtocolMatch> {
                let inner_proto = std::sync::Arc::new(self.inner.clone());
                inner_proto.identify(data).map(|m| {
                    $crate::core::types::ProtocolMatch {
                        name: m.name,
                        metadata: m.metadata,
                        implementation: self,
                    }
                })
            }

            fn name(&self) -> String {
                use $crate::heck::ToSnakeCase;
                stringify!($proto).to_snake_case()
            }

            fn transport(&self) -> $crate::core::types::Transport {
                use $crate::core::types::RefractiumProtocol;
                self.inner.transport()
            }

            fn hooks(&self) -> Vec<std::sync::Arc<dyn $crate::protocols::hooks::ProtocolHook>> {
                self.hooks.clone()
            }
        }

        impl Default for $wrapper {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

/// Internal trace logging macro.
///
/// Only active when the `logging` feature is enabled.
macro_rules! refractium_trace {
    ($($arg:tt)*) => {
        {
            #[cfg(feature = "logging")]
            tracing::trace!($($arg)*);
        }
    };
}

/// Internal debug logging macro.
///
/// Only active when the `logging` feature is enabled.
macro_rules! refractium_debug {
    ($($arg:tt)*) => {
        {
            #[cfg(feature = "logging")]
            tracing::debug!($($arg)*);
        }
    };
}

/// Internal info logging macro.
///
/// Only active when the `logging` feature is enabled.
macro_rules! refractium_info {
    ($($arg:tt)*) => {
        {
            #[cfg(feature = "logging")]
            tracing::info!($($arg)*);
        }
    };
}

/// Internal warning logging macro.
///
/// Only active when the `logging` feature is enabled.
macro_rules! refractium_warn {
    ($($arg:tt)*) => {
        {
            #[cfg(feature = "logging")]
            tracing::warn!($($arg)*);
        }
    };
}

/// Internal error logging macro.
///
/// Only active when the `logging` feature is enabled.
macro_rules! refractium_error {
    ($($arg:tt)*) => {
        {
            #[cfg(feature = "logging")]
            tracing::error!($($arg)*);
        }
    };
}

pub(crate) use refractium_debug;
pub(crate) use refractium_error;
pub(crate) use refractium_info;
pub(crate) use refractium_trace;
pub(crate) use refractium_warn;
