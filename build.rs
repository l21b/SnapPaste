fn main() {
    println!("cargo:rerun-if-changed=ui/main.slint");
    println!("cargo:rerun-if-changed=ui/icons");
    println!("cargo:rerun-if-changed=icons/icon.ico");
    slint_build::compile("ui/main.slint").expect("failed to compile Slint UI");

    #[cfg(target_os = "windows")]
    {
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon("icons/icon.ico")
            .set("ProductName", "SnapPaste")
            .set("FileDescription", "SnapPaste Clipboard Manager")
            .set("CompanyName", "21b")
            .set("LegalCopyright", "Copyright © 21b");
        resource
            .compile()
            .expect("failed to compile Windows resources");
    }
}
