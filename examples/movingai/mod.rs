use std::fs::File;
use std::io::{BufRead, BufReader, Error, Result};
use std::path::{Path, PathBuf};

use mkpath::grid::BitGrid;

pub struct Problem {
    pub bucket: u32,
    pub start: (i32, i32),
    pub target: (i32, i32),
    pub optimal: f64,
}

pub struct Scenario {
    pub map: PathBuf,
    pub instances: Vec<Problem>,
}

pub fn read_scenario(scen_path: &Path) -> Result<Scenario> {
    let mut lines = BufReader::new(File::open(scen_path)?).lines();
    check_version(lines.next().transpose()?)?;

    let mut map = None;
    let mut map_size = None;
    let mut instances = vec![];

    for line in lines {
        let line = line?;
        let mut tokens = line.split_whitespace();

        let Some(bucket) = tokens.next() else {
            continue;
        };
        let bucket = bucket.parse().map_err(Error::other)?;

        let problem_map = tokens
            .next()
            .ok_or_else(|| Error::other("problem instance missing field map"))?;

        let mut next_int = |field: &str| {
            tokens
                .next()
                .ok_or_else(|| Error::other(format!("problem instance missing field {field}")))?
                .parse()
                .map_err(Error::other)
        };

        let map_width = next_int("map width")?;
        let map_height = next_int("map height")?;
        let start_x = next_int("start x")?;
        let start_y = next_int("start y")?;
        let target_x = next_int("goal x")?;
        let target_y = next_int("goal y")?;

        let optimal = tokens
            .next()
            .ok_or_else(|| {
                Error::other("problem instance missing field optimal length".to_string())
            })?
            .parse()
            .map_err(Error::other)?;

        if let Some(map) = &map {
            if problem_map != map {
                return Err(Error::other("problem instance specifies different map"));
            }
            if Some((map_width, map_height)) != map_size {
                return Err(Error::other(
                    "problem instance specifies incorrect map size",
                ));
            }
        } else {
            map = Some(problem_map.to_owned());
            map_size = Some((map_width, map_height));
        }

        instances.push(Problem {
            bucket,
            start: (start_x, start_y),
            target: (target_x, target_y),
            optimal,
        });
    }

    Ok(Scenario {
        map: locate_map(&map.unwrap(), scen_path)?,
        instances,
    })
}

fn locate_map(map_path: &str, scen_path: &Path) -> Result<PathBuf> {
    let location_1 = scen_path.parent().unwrap().join(map_path);
    if location_1.try_exists()? {
        return Ok(location_1);
    }
    Ok(Path::new(map_path).to_path_buf())
}

fn field(line: Option<&str>) -> Result<(&str, &str)> {
    let Some(line) = line else {
        return Err(Error::other("unexpected end of file"));
    };
    let mut tokens = line.split_whitespace();

    let Some(tok1) = tokens.next() else {
        return Err(Error::other("unexpected end of line"));
    };
    let Some(tok2) = tokens.next() else {
        return Err(Error::other("unexpected end of line"));
    };
    let None = tokens.next() else {
        return Err(Error::other("unexpected trailing text"));
    };

    Ok((tok1, tok2))
}

fn check_version(version_line: Option<String>) -> Result<()> {
    let (version, number) = field(version_line.as_deref())?;

    if version != "version" {
        return Err(Error::other(format!("expected version, got {version}")));
    }

    if number != "1" && number != "1.0" {
        return Err(Error::other(format!(
            "unsupported version number: {number}"
        )));
    }

    Ok(())
}

pub fn read_bitgrid(map: &Path) -> Result<BitGrid> {
    let mut lines = BufReader::new(File::open(map)?).lines();

    let type_line = lines.next().transpose()?;
    let (type_, octile) = field(type_line.as_deref())?;

    if type_ != "type" {
        return Err(Error::other("expected first line to be type"));
    }
    if octile != "octile" {
        return Err(Error::other("expected type to be octile"));
    }

    let height_line = lines.next().transpose()?;
    let (height, y) = field(height_line.as_deref())?;

    if height != "height" {
        return Err(Error::other("expected second line to be height"));
    }
    let y = y.parse().map_err(Error::other)?;

    let width_line = lines.next().transpose()?;
    let (width, x) = field(width_line.as_deref())?;

    if width != "width" {
        return Err(Error::other("expected third line to be width"));
    }
    let x = x.parse().map_err(Error::other)?;

    if lines.next().transpose()?.as_deref() != Some("map") {
        return Err(Error::other("expected map token"));
    }

    let mut map = BitGrid::new(x, y);

    for (y, row) in lines.enumerate() {
        let row = row?;
        if y as i32 >= map.height() {
            return Err(Error::other("too many lines of map"));
        }
        for (x, cell) in row.chars().enumerate() {
            if x as i32 >= map.width() {
                return Err(Error::other("too many columns of map"));
            }
            map.set(x as i32, y as i32, matches!(cell, '.' | 'G' | 'S'));
        }
    }

    Ok(map)
}
