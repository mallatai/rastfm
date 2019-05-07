use std::io::Read;
use std::fs::File;
use std::path::Path;
use regex::{Regex, Captures};

// #[derive(Debug, Deserialize)]
pub struct TrackInfo {
    pub artist: String,
    pub track_name: String,
    pub timestamp: u64,
    pub album: Option<String>,
    pub track_number: Option<u16>,
    pub duration: Option<u32>,
    pub status: Option<String>,
    pub uid: Option<String>
}

pub fn parse() -> Vec<TrackInfo> {
    let mut track_info_list = Vec::new();

    let track_regex = Regex::new(r"(?x)
                                 ^(?P<artist>[^\t]+)\t
                                 (?P<album>[^\t]+)\t
                                 (?P<track_name>[^\t]+)\t
                                 (?P<track_number>[^\t]+)\t
                                 (?P<duration>[^\t]+)\t
                                 (?P<status>[^\t]+)\t
                                 (?P<timestamp>[^\t]+)\t
                                 (?P<uid>[^\t]+)?$"
                                 ).unwrap();
    let log_file_name = "scrobbler.log";

    let scrobbler_log_path = Path::new(log_file_name);
    if !(scrobbler_log_path.exists() && scrobbler_log_path.is_file()) {
        return track_info_list;
    }

    let mut log_file = File::open(scrobbler_log_path).unwrap();
    let mut log_string = String::new();
    let _ = log_file.read_to_string(&mut log_string);

    track_info_list = log_string.lines()
        .map(|line| track_regex.captures(line))
        .filter(|capture| capture.is_some())
        .map(|capture| extract_track_info(&capture.unwrap()))
        .filter(|track_info| track_info.is_some())
        .map(|track_info| track_info.unwrap())
        .collect::<Vec<_>>();

    track_info_list
}

fn extract_track_info<'a>(captures: &Captures<'a>) -> Option<TrackInfo> {
    let artist = captures.name("artist");
    let track_name = captures.name("track_name");
    let timestamp = captures.name("timestamp");
    if artist.is_some() && track_name.is_some() && timestamp.is_some() {
        let album = captures.name("album");
        let track_number = captures.name("track_number").map(|m| m.as_str()).and_then(|s| s.parse::<u16>().ok());
        let duration = captures.name("duration").map(|m| m.as_str()).and_then(|s| s.parse::<u32>().ok());
        let status = captures.name("status");
        let uid = captures.name("uid");

        Some( TrackInfo { artist: artist.unwrap().as_str().to_owned(),
                          track_name: track_name.unwrap().as_str().to_owned(),
                          timestamp: timestamp.map(|m| m.as_str()).and_then(|s| s.parse::<u64>().ok()).unwrap(),
                          album: album.map(|s| s.as_str().to_owned()),
                          track_number: track_number,
                          duration: duration,
                          status: status.map(|s| s.as_str().to_owned()),
                          uid: uid.map(|s| s.as_str().to_owned()) } )
    } else {
        None
    }
}

