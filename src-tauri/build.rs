fn main() {
    println!("cargo:rerun-if-env-changed=PIEALL_TAURI_PUBLIC_KEY");
    tauri_build::build()
}
