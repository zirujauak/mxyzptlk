extern crate simplelog;

use std::fs::File;

use simplelog::*;

pub fn init() {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();

    // WriteLogger::init(
    //     LevelFilter::Trace,
    //     Config::default(),
    //     File::create("trace.log").unwrap(),
    // )
    // .unwrap();

    trace!("Log start");
}
