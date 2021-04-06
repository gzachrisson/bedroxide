pub fn to_hex(buf: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    for &byte in buf.iter() {
        write!(&mut s, "{:02X} ", byte).expect("Unable to write");
    }
    return s;
}    
