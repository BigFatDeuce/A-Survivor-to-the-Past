//! Icone et informations de l'executable Windows.
fn main() {
    println!("cargo:rerun-if-changed=../../assets/icons/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../../assets/icons/icon.ico")
        .set("ProductName", "A Survivor to the Past")
        .set("FileDescription", "A Survivor to the Past")
        .set("LegalCopyright", "Fangame non commercial - assets (c) Nintendo");
    if let Err(e) = res.compile() {
        println!("cargo:warning=icone de l'exe non integree : {e}");
    }
}
