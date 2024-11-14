//! WASI 

pub mod async_impl;
pub mod blocking;

#[allow(missing_docs)]
pub mod wit {
    wit_bindgen::generate!({
        path: "wit",
        world: "reqwest",
    });
    // wit_bindgen::generate!("http-client");
}
