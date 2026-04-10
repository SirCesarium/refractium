// @swt-disable max-repetition

/// Macro to define a new protocol by implementing the `PrismaProtocol` trait.
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
        pub struct $name;

        impl $crate::protocols::PrismaProtocol for $name {
            #[inline]
            fn identify(&self, $data: &[u8]) -> Option<$crate::protocols::ProtocolMatch> {
                if $body {
                    return Some($crate::protocols::ProtocolMatch {
                        name: stringify!($name).to_lowercase(),
                        metadata: None,
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

/// Internal debug logging macro.
///
/// Only active when the `logging` feature is enabled.
#[macro_export]
macro_rules! prisma_debug {
    ($($arg:tt)*) => {
        {
            #[cfg(feature = "logging")]
            tracing::info!($($arg)*);
        }
    };
}

/// Internal error logging macro.
///
/// Only active when the `logging` feature is enabled.
#[macro_export]
macro_rules! prisma_error {
    ($($arg:tt)*) => {
        {
            #[cfg(feature = "logging")]
            tracing::error!($($arg)*);
        }
    };
}
