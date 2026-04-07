use crate::protocols::Protocol;

pub struct PrismaCore {
    protocols: Vec<Box<dyn Protocol>>,
    fallback: String,
}

impl PrismaCore {
    #[must_use]
    pub fn new(fallback: String) -> Self {
        Self {
            protocols: Vec::new(),
            fallback,
        }
    }

    pub fn register<P: Protocol + 'static>(&mut self, proto: P) {
        self.protocols.push(Box::new(proto));
    }

    #[must_use]
    pub fn resolve(&self, data: &[u8]) -> &str {
        for proto in &self.protocols {
            if proto.identify(data) {
                return proto.target_addr();
            }
        }
        &self.fallback
    }
}
