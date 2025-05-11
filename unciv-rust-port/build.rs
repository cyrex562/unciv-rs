fn main() {
    println!("cargo:rustc-link-search=native=../vendors/SDL3/lib/x64");
    println!("cargo:rustc-link-lib=SDL3");
    println!("cargo:include=../vendors/SDL3/include");
}