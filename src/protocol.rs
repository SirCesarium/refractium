pub enum Protocol {
    Http,
    Binary,
}

#[must_use]
pub fn identify(buf: &[u8]) -> Protocol {
    let len = buf.len();
    if len < 3 {
        return Protocol::Binary;
    }

    let is_http = match buf {
        _ if len >= 4 && (&buf[0..4] == b"GET " || &buf[0..4] == b"PUT ") => true,
        _ if len >= 5 && (&buf[0..5] == b"POST " || &buf[0..5] == b"HEAD ") => true,
        _ if len >= 6 && &buf[0..6] == b"PATCH " => true,
        _ if len >= 7 && &buf[0..7] == b"DELETE " => true,
        _ if len >= 8 && &buf[0..8] == b"OPTIONS " => true,
        _ => false,
    };

    if is_http {
        Protocol::Http
    } else {
        Protocol::Binary
    }
}
