extern crate regex;
extern crate hyper;
extern crate serde_json;
extern crate serde_urlencoded;
extern crate crypto;
extern crate futures;
extern crate tokio_core;

mod rastfm;

fn main() {
    println!("Starting...");
    let mut track_info_list = rastfm::logfile::parse();
    println!("Total tracks collected: {}", track_info_list.len());

    rastfm::lastfm_api::scrobble_tracks(&mut track_info_list);
}

