fn main() {
    println!("cargo:rerun-if-changed=../../assets/keyconfig.rc");
    println!("cargo:rerun-if-changed=../../assets/icon/keyconfig.ico");
    embed_resource::compile("../../assets/keyconfig.rc", embed_resource::NONE);
}
