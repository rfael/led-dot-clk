use std::env;

fn main() {
    println!("cargo:rustc-link-arg-bins=-Tlinkall.x");

    if let Ok(f) = env::var("CARGO_CFG_FEATURE")
        && f.contains("defmt")
    {
        println!("cargo:rustc-link-arg-bins=-Tdefmt.x");
    }
}
