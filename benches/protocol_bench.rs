use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use rand::rng;
use rand::seq::IndexedRandom;
use refractium::protocols::DynamicProtocol;
use refractium::protocols::ProtocolRegistry;
use refractium::protocols::dns::Dns;
use refractium::protocols::ftp::Ftp;
use refractium::protocols::http::Http;
use refractium::protocols::https::Https;
use refractium::protocols::ssh::Ssh;

fn setup_full_registry() -> ProtocolRegistry {
    let mut registry = ProtocolRegistry::new();
    registry.register(Box::new(Http));
    registry.register(Box::new(Https));
    registry.register(Box::new(Ssh));
    registry.register(Box::new(Ftp));
    registry.register(Box::new(Dns));
    registry.register(Box::new(DynamicProtocol {
        name: "Minecraft".to_string(),
        patterns: vec![
            "\x00\x04\x03\x01\x07".to_string(),
            "\u{00fe}\x01".to_string(),
        ],
    }));
    registry
}

fn bench_realistic_scenarios(c: &mut Criterion) {
    let registry = setup_full_registry();

    let payloads = [
        b"GET /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n".to_vec(),
        b"SSH-2.0-OpenSSH_8.2p1 Ubuntu-4ubuntu0.1\r\n".to_vec(),
        vec![
            0x16, 0x03, 0x01, 0x00, 0xba, 0x01, 0x00, 0x00, 0xb6, 0x03, 0x03, 0xfa, 0xaf, 0xaf,
            0xaf, 0xaf, 0xaf, 0xaf, 0xaf, 0xaf, 0xaf, 0xaf, 0xaf, 0xaf, 0xaf, 0xaf, 0xaf, 0xaf,
            0xaf, 0xaf, 0xaf, 0xaf, 0xaf, 0xaf, 0xaf, 0xaf, 0xaf, 0xaf, 0xaf, 0xaf, 0xaf, 0xaf,
            0xaf, 0x00, 0x00, 0x02, 0x00, 0x2f, 0x01, 0x00, 0x00, 0x8b, 0x00, 0x00, 0x00, 0x0f,
            0x00, 0x0d, 0x00, 0x00, 0x0a, 0x67, 0x6f, 0x6f, 0x67, 0x6c, 0x65, 0x2e, 0x63, 0x6f,
            0x6d,
        ],
        vec![
            0x24, 0x1a, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x65,
            0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x03, 0x63, 0x6f, 0x6d, 0x00, 0x00, 0x01, 0x00,
            0x01,
        ],
        b"USER anonymous\r\n".to_vec(),
        vec![
            0xfe, 0x01, 0xfa, 0x00, 0x0b, 0x00, 0x4d, 0x00, 0x43, 0x00, 0x7c, 0x00, 0x50, 0x00,
            0x69, 0x00, 0x6e, 0x00, 0x67, 0x00, 0x48, 0x00, 0x6f, 0x00, 0x73, 0x00, 0x74,
        ],
        (0..64).map(|_| rand::random::<u8>()).collect(),
    ];

    // Randomized probe distribution
    c.bench_function("mixed_traffic", |b| {
        let mut r = rng();
        b.iter(|| {
            let payload = payloads.choose(&mut r).unwrap();
            black_box(registry.probe(black_box(payload)));
        });
    });

    // Full registry scan on invalid data
    c.bench_function("worst_case_scan", |b| {
        let garbage = vec![0u8; 128];
        b.iter(|| {
            black_box(registry.probe(black_box(&garbage)));
        });
    });

    // Complex TLS extension parsing
    c.bench_function("https_sni_extraction", |b| {
        let tls = &payloads[2];
        b.iter(|| {
            black_box(registry.probe(black_box(tls)));
        });
    });

    // Individual protocol performance
    for (name, payload) in [
        ("http", &payloads[0]),
        ("ssh", &payloads[1]),
        ("dns", &payloads[3]),
        ("ftp", &payloads[4]),
    ] {
        c.bench_function(format!("probe_{name}").as_str(), |b| {
            b.iter(|| {
                black_box(registry.probe(black_box(payload)));
            });
        });
    }
}

criterion_group!(benches, bench_realistic_scenarios);
criterion_main!(benches);
