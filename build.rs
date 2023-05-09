use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const I18N_DIR: &str = "i18n";
// https://github.com/rust-lang/cargo/issues/5457
const TARGET_DIR: &str = "target";

fn main() {
    generate_translation_files();
}

fn generate_translation_files() {
    let mut dest_path = PathBuf::from(TARGET_DIR);
    dest_path.push("locale");

    println!("Creating directory: {:?}", &dest_path);
    let _ = fs::remove_dir_all(&dest_path);
    fs::create_dir(&dest_path).expect("Could not create translations directory");

    println!("cargo:rerun-if-changed={}", I18N_DIR);
    let existing_iter = fs::read_dir(I18N_DIR)
        .unwrap()
        .filter(|x| x.as_ref().unwrap().path().extension().unwrap() == "po");

    for existing_file in existing_iter {
        let file = existing_file.unwrap();
        generate_mo(&file.path(), &dest_path)
    }
}

fn generate_mo(src_path: &Path, dest_path: &Path) {
    let mut file_dest_path = dest_path.to_path_buf();
    file_dest_path.push(src_path.file_stem().unwrap());
    file_dest_path.push("LC_MESSAGES");
    fs::create_dir_all(&file_dest_path).expect("Could not create translations directory");
    file_dest_path.push("tundra.mo");

    print!(
        "Generating .mo file for {:?} to {} ",
        &src_path.display(),
        file_dest_path.display(),
    );
    let res = Command::new("msgfmt")
        .arg(format!("--output-file={}", file_dest_path.display()))
        .arg(src_path.to_str().unwrap())
        .output()
        .expect("Could not execute msgfmt");

    if res.status.success() {
        println!("success");
    } else {
        panic!(
            "msgfmt exited with a non-zero status code {}: {:?}",
            res.status, res
        );
    }

    println!("cargo:rerun-if-changed={}", src_path.display());
}
