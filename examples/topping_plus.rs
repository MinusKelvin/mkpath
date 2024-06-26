use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;

use clap::Parser;
use mkpath_grid_gb::{PartialCellCpd, ToppingPlus};
use mkpath_jps::JumpDatabase;

mod movingai;

#[derive(Parser)]
struct Options {
    path: PathBuf,
    #[arg(long)]
    generate: bool,
}

fn main() {
    let opt = Options::parse();

    if opt.generate {
        let mut cpd_file = opt.path.clone();
        cpd_file.as_mut_os_string().push(".top+");

        let map = movingai::read_bitgrid(&opt.path).unwrap();
        let jump_db = JumpDatabase::new(&map);

        let mut file = BufWriter::new(File::create(cpd_file).unwrap());

        PartialCellCpd::compute_to_file(&map, &jump_db, &mut file, |progress, total, time| {
            let done = progress == total;
            let progress = progress as f64 / total as f64;
            let ttg = if done {
                time.as_secs_f64() as u64
            } else {
                (time.as_secs_f64() / progress - time.as_secs_f64()) as u64
            };
            let mut stdout = std::io::stdout().lock();
            let _ = write!(
                stdout,
                "\r{:4.1}% {} {} hr {:2} min {:2} sec",
                (progress * 1000.0).round() / 10.0,
                if done { "Done" } else { "ETA" },
                ttg / 60 / 60,
                ttg / 60 % 60,
                ttg % 60,
            );
            stdout.flush().unwrap();
        })
        .unwrap();
        println!();
    } else {
        let t1 = std::time::Instant::now();

        let scen = movingai::read_scenario(&opt.path).unwrap();
        let map = movingai::read_bitgrid(&scen.map).unwrap();
        let jump_db = JumpDatabase::new(&map);

        let mut cpd_file = scen.map.clone();
        cpd_file.as_mut_os_string().push(".top+");
        let oracle =
            PartialCellCpd::load(&map, &mut BufReader::new(File::open(cpd_file).unwrap())).unwrap();
        let mut topping_plus = ToppingPlus::new(&map, &jump_db, &oracle);

        let t2 = std::time::Instant::now();

        for problem in &scen.instances {
            let (path, cost) = topping_plus.get_path(problem.start, problem.target);
            println!("{cost:.2} {path:?}");
        }

        let t3 = std::time::Instant::now();
        eprintln!("Load: {:<10.2?} Search: {:.2?}", t2 - t1, t3 - t2);
    }
}
