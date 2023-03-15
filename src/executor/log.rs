pub fn init(name: &String) {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    trace!("Start trace log for '{}'", name);
    info!(target: "app::frame", "Start frame log for '{}'", name);
    info!(target: "app::instruction", "Start instruction log for '{}'", name);
    info!(target: "app::memory", "Start memory log for '{}'", name);
    info!(target: "app::stack", "Start stack log for '{}'", name);
    info!(target: "app::variable", "Start variable log for '{}'", name);
}
