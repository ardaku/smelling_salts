/*#[macro_use]
extern crate stdweb;
extern crate wasm_bindgen;

use wasm_bindgen::prelude::*;*/

extern "C" {
    fn cala_test() -> ();
//    fn alert(value: u32) -> ();
}

fn main() {
    unsafe {
        cala_test();
    }

   /* js! {
        alert("YO!!");
    }*/

   // unsafe {
        // alert(b"hello world\0".as_ptr());
     //   test();
    //}
    // web_sys::window().unwrap().alert_with_message("Hello, world!");
}
