fn main() {
    #[cfg(not(target_os = "none"))]
    uniffi::generate_scaffolding("./src/lib.udl").unwrap();
}
