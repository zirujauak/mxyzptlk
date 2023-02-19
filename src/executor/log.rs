extern crate simplelog;

use std::fs::File;

use simplelog::*;

pub fn init() {
    WriteLogger::init(
        LevelFilter::Trace,
        Config::default(),
        File::create("trace.log").unwrap(),
    )
    .unwrap();

    trace!("Log start");
}
