extern crate simplelog;

pub fn init() {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    trace!("Log start");
}
