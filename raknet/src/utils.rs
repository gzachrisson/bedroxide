pub fn to_hex(buf: &[u8], max_bytes: usize) -> String {
    use std::fmt::Write;
    let buf = &buf[..buf.len().min(max_bytes)];
    let mut s = String::new();
    for &byte in buf.iter() {
        write!(&mut s, "{:02X} ", byte).expect("Unable to write");
    }
    return s;
}    
