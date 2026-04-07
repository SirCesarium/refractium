pub enum Transport {
    Tcp,
    Udp,
}

pub trait Protocol: Send + Sync {
    fn name(&self) -> &'static str;
    fn transport(&self) -> Transport;
    fn identify(&self, buf: &[u8]) -> bool;
    fn target_addr(&self) -> &str;
}
