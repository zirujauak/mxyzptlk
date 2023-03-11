pub fn init(name: &String) {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    trace!("Start trace log for '{}'", name);
    info!(target: "app::instruction", "Start instruction log for '{}'", name);
}
