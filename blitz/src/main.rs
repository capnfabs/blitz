#[macro_use]
extern crate clap;

use std::fs::File;
use std::io::Write;

fn main() {
    let matches = clap_app!(blitz =>
        (version: "1.0")
        (author: "Fabian Tamp (https://capnfabs.net/contact)")
        (about: "Does awesome things")
        (@arg CONFIG: -c --config +takes_value "Sets a custom config file")
        (@arg INPUT: +required "Sets the input file to use")
        (@arg debug: -d ... "Sets the level of debugging information")
        (@subcommand test =>
            (about: "controls testing features")
            (version: "1.3")
            (author: "Someone E. <someone_else@other.com>")
            (@arg verbose: -v --verbose "Print test information verbosely")
        )
    ).get_matches();

    let preview_filename = "/tmp/thumb.jpg";
    let file = libraw::RawFile::open(matches.value_of("INPUT").unwrap()).unwrap();
    println!("Opened file: {:?}", file);
    dump_to_file(preview_filename, file.get_jpeg_thumbnail()).unwrap();
    open_preview(preview_filename)
}

fn dump_to_file(filename: &str, data: &[u8]) -> std::io::Result<()> {
    let mut file = File::create(filename)?;
    println!("Writing {} bytes to {}", data.len(), filename);
    file.write_all(data)?;
    Ok(())
}

fn open_preview(filename: &str) {
    use std::process::Command;

    let output = Command::new("open")
        .arg(filename)
        .spawn()
        .expect("Failed to start");
}
