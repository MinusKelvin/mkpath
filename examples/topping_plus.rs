use std::io::Write;
use std::path::{Path, PathBuf};

use mkpath_topping::ToppingPlusOracle;
use structopt::StructOpt;

mod movingai;

#[derive(StructOpt)]
struct Options {
    path: PathBuf,
    #[structopt(long)]
    generate: bool,
}

fn main() {
    let opt = Options::from_args();

    if opt.generate {
        let mut cpd_file = opt.path.clone();
        cpd_file.as_mut_os_string().push(".top+");

        let map = movingai::read_bitgrid(&opt.path).unwrap();
        let states: usize = (0..map.height())
            .map(|y| (0..map.width()).filter(|&x| map.get(x, y)).count())
            .sum();
        let oracle = ToppingPlusOracle::compute(map, |progress, total, time| {
            if progress & 0xF == 0 {
                let progress = progress as f64 / total as f64;
                let ttg = (time.as_secs_f64() / progress - time.as_secs_f64()) as u64;
                print!(
                    "\r{:4.1}% ETA {} hr {:2} min {:2} sec     {states} {total}",
                    (progress * 1000.0).round() / 10.0,
                    ttg / 60 / 60,
                    ttg / 60 % 60,
                    ttg % 60,
                );
                std::io::stdout().flush().unwrap();
            }
        });
    } else {
        let scen = movingai::read_scenario(&opt.path).unwrap();
        let map = movingai::read_bitgrid(&scen.map).unwrap();
        let mut cpd_file = scen.map.clone();
        cpd_file.as_mut_os_string().push(".top+");
        // let (mapper, rows) = load_cpd(&cpd_file, map.width(), map.height()).unwrap();

        for problem in scen.instances {}
    }
}
