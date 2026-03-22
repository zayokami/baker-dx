use log::info;

fn main() {
    log4rs::init_file("server/log4rs.yaml", Default::default()).unwrap();

    info!("----------------------------------------");
    info!("Baker-Dx Server");
    info!("");
    info!("Version {}", env!("CARGO_PKG_VERSION"));
    info!("");
    info!("----------------------------------------");
}
