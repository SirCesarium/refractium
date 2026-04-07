#[macro_export]
macro_rules! define_protocol {
    ($struct_name:ident, $display_name:expr, $transport:expr, $target:expr, $magic:expr) => {
        pub struct $struct_name;
        impl $crate::protocols::Protocol for $struct_name {
            fn name(&self) -> &'static str {
                $display_name
            }
            fn transport(&self) -> $crate::protocols::Transport {
                $transport
            }
            fn target_addr(&self) -> &str {
                $target
            }
            fn identify(&self, buf: &[u8]) -> bool {
                buf.starts_with($magic)
            }
        }
    };
}
