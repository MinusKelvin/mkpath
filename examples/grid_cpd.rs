use std::ffi::OsStr;
use std::path::PathBuf;

use mkpath::grid::{eight_connected, GridPool};
use mkpath::{NodeBuilder, PriorityQueueFactory};
use structopt::StructOpt;

mod movingai;

#[derive(StructOpt)]
enum Options {
    Generate {
        map: PathBuf,
        data_path: Option<PathBuf>
    },
    Solve {
        scen: PathBuf,
    }
}

fn main() {
    let opt = Options::from_args();

    match opt {
        Options::Generate { map, data_path } => {
            let map_file_name = map.file_name().unwrap();
            let data_file = match data_path {
                Some(path) => {
                    if path.is_dir() {
                        path.join(map_file_name)
                    } else {
                        path
                    }
                }
                None => {
                    let mut map_file_name = map_file_name.to_os_string();
                    map_file_name.push(OsStr::new(".cpd"));
                    map.with_file_name(map_file_name)
                }
            };
            build_cpd(map, data_file);
        }
        Options::Solve { scen } => todo!(),
    }
}
