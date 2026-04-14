// @swt-disable max-repetition

/// Macro to define a new protocol by implementing the `RefractiumProtocol` trait.
///
/// This macro simplifies the creation of simple protocols that identify themselves
/// by checking a data slice against a condition.
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

        impl $crate::protocols::RefractiumProtocol for $name {
            #[inline]
            fn identify(self: std::sync::Arc<Self>, $data: &[u8]) -> Option<$crate::protocols::ProtocolMatch> {
                if $body {
                    return Some($crate::protocols::ProtocolMatch {
                        name: stringify!($name).to_lowercase(),
                        metadata: None,
                        implementation: self,
                    });
                }
                None
            }

            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn transport(&self) -> $crate::core::types::Transport {
                $transport
            }
        }
    };
}

/// Macro to quickly define a new protocol hook.
#[macro_export]
macro_rules! define_hook {
    ($name:ident, |$dir:ident, $pkt:ident| $body:expr) => {
        #[derive(Clone)]
        pub struct $name;
        impl $crate::protocols::hooks::ProtocolHook for $name {
            fn name(&self) -> &'static str {
                stringify!($name)
            }
            fn on_packet(
                &self,
                $dir: $crate::protocols::hooks::Direction,
                $pkt: $crate::bytes::Bytes,
            ) {
                $body
            }
        }
    };
}

/// Automatically generated wrapper to intercept protocol traffic.
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
            pub fn new() -> Self {
                Self {
                    inner: $proto,
                    hooks: vec![$(std::sync::Arc::new($hook)),*],
                }
            }

            pub fn with_hooks(hooks: Vec<std::sync::Arc<dyn $crate::protocols::hooks::ProtocolHook>>) -> Self {
                Self {
                    inner: $proto,
                    hooks,
                }
            }
        }

        impl $crate::protocols::RefractiumProtocol for $wrapper {
            #[inline]
            fn identify(self: std::sync::Arc<Self>, data: &[u8]) -> Option<$crate::protocols::ProtocolMatch> {
                let inner_proto = std::sync::Arc::new(self.inner.clone());
                inner_proto.identify(data).map(|m| {
                    $crate::protocols::ProtocolMatch {
                        name: m.name,
                        metadata: m.metadata,
                        implementation: self,
                    }
                })
            }

            fn name(&self) -> &'static str {
                stringify!($proto)
            }

            fn transport(&self) -> $crate::core::types::Transport {
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
#[macro_export]
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
#[macro_export]
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
#[macro_export]
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
#[macro_export]
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
#[macro_export]
macro_rules! refractium_error {
    ($($arg:tt)*) => {
        {
            #[cfg(feature = "logging")]
            tracing::error!($($arg)*);
        }
    };
}
