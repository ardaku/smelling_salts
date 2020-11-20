extern "C" {
    fn cala_alert(value: u32);
}

pub extern "Rust" fn cala_rust_alert(value: u32) {
    unsafe {
        cala_alert(value);
    }
}
