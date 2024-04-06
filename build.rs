fn main() {
    // shared::grammar::fetch_grammars();
    // shared::grammar::build_grammars();

    println!("cargo:rerun-if-changed=build.rs");
}
