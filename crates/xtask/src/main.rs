mod flags {
    use std::path::PathBuf;

    xflags::xflags! {

    cmd my-command {
        required path: PathBuf
        optional -v, --verbose
    }
    }
}

fn main() {
    let flags = flags::MyCommand::from_env();
    println!("{:#?}", flags);
}
