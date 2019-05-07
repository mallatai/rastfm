extern crate serde_json;

use std::fs::{File, remove_file};
use std::path::Path;
use std::io::{Read, Write};
use std::process::Command;
use std::collections::HashMap;
use std::{thread, time};

use tokio_core::reactor::Core;
use futures::{Future, Stream};
use futures::future;
use hyper::client::{Client, Request, Response};
use hyper::Method;
use hyper::StatusCode;
use hyper::header::{ContentLength, ContentType};
use hyper::Error;

use serde_json::Value;
use serde_urlencoded;

use crypto::md5::Md5;
use crypto::digest::Digest;

use rastfm::logfile::TrackInfo;

const SESSION_KEY_FILE: &str = "session_key";
const API_KEY: &str = "";

pub struct ApiCredentials {
    username: String,
    password: String,
    api_key: String,
    api_secret: String,
}

pub struct AuthorizedApiCredentials {
    username: String,
    password: String,
    api_key: String,
    api_secret: String,
    session_key: Option<String>,
}

// TODO remove unnecessary unwraps
// TODO use Result
// TODO maybe use method syntax and traits
// TODO use https://crates.io/crates/md5 instead of rust_crypto

pub fn get_session(api_key: &str) -> Option<String> {
    let session_key = load_session_key();
    if session_key.is_some() {
        return session_key;
    }

    let get_token_url = format!("http://ws.audioscrobbler.com/2.0/?method=auth.getToken&api_key={}&format=json", api_key).parse().unwrap();
    let mut core = Core::new().unwrap();
    let client = Client::new(&core.handle());
    let mut work = client.get(get_token_url).and_then(|res| {
        println!("Response status: {}", res.status());
        res.body().concat2().and_then(move |body| {
            let response_json: Value = serde_json::from_slice(&body).expect("from_slice error");
            let token = response_json["token"].as_str().unwrap().to_string();
            Ok(token)
        })
    });
    let mut auth_token = core.run(work).unwrap();
    println!("Got token: {0}", auth_token);

    let auth_link = format!("http://www.last.fm/api/auth/?api_key={}&token={}", api_key, auth_token);
    let mut status = Command::new("/usr/bin/xdg-open")
                               .arg(auth_link)
                               .status()
                               .expect("Failed to launch web-browser");

    thread::sleep(time::Duration::from_secs(15));

    if status.success() {
        let mut params = Vec::new();
        params.push(("method", "auth.getSession"));
        params.push(("api_key", api_key));
        params.push(("token", auth_token.as_str()));
        let session_url = build_url(&params).parse().unwrap();
        println!("Session url: {}", session_url);

        let mut work = client.get(session_url).and_then(move |res| {
            println!("Response status: {}", res.status());
            res.body().concat2().and_then(move |body| {
                let response_json: Value = serde_json::from_slice(&body).expect("from_slice error");
                let ref session_value = response_json["session"];
                let key = session_value["key"].as_str().unwrap().to_string();
                println!("Got session: {}", key);
                save_session_key(key.as_str());

                Ok(key.to_owned())
            })
        });

        let session_key = core.run(work).unwrap();

        return Some(session_key);
    }

    None
}

pub fn scrobble_tracks(tracks: &mut Vec<TrackInfo>) {
    let mut params = Vec::new();
    let mut i: u16 = 0;
    let tracks_len = tracks.len() as u16;
    while let Some(track) = tracks.pop() {
        let track_params = make_track_params(i, &track);
        for &(ref k, ref v) in &track_params {
            params.push((k.to_string(), v.to_string()));
        }

        i = i + 1;
        if i >= 9 || i >= tracks_len { // TODO update current tracks_len variable to the number of the remaining tracks
            scrobble(&params);
            i = 0;
            params.clear();
        }
        println!("{}", i);
    }
}

fn scrobble(params: &Vec<(String, String)>) {
    let sk = get_session("<your_session_key>").expect("Couldn't load session key from a file");
    let mut params_slice = Vec::new();
    params_slice.push(("method", "track.scrobble"));
    params_slice.push(("api_key", API_KEY));
    params_slice.push(("sk", &sk));

    let url_sig = build_signature(&params_slice);

    for &(ref k, ref v) in params {
        params_slice.push((k.as_str(), v.as_str()));
    }

    let req_data = make_req_body_urlencoded(&params_slice).expect("Couldn't make urlencoded request body");
    let scrobble_url = format!("http://ws.audioscrobbler.com/2.0/?method=track.scrobble&api_key={}&format=json", API_KEY);
    // let scrobble_url = format!("http://ws.audioscrobbler.com/2.0/?method=track.scrobble&sk={}&api_key={}&api_sig={}&format=json", sk, API_KEY, url_sig);
    let mut req = Request::new(Method::Post, scrobble_url.parse().unwrap());
    // req.headers_mut().set(ContentType::form_url_encoded());
    req.set_body(req_data);

    let mut core = Core::new().unwrap();
    let client = Client::new(&core.handle());
    let mut work = client.request(req).and_then(move |res| {
        println!("Response status: {}", res.status());
        if res.status() == StatusCode::Ok {
            res.body().concat2().and_then(move |body| {
                let response_json: Value = serde_json::from_slice(&body).expect("from_slice error");
                println!("Got response: {}", response_json.to_string());

                Ok(())
            });
        }
        Ok(())
    });

    let _ = core.run(work).unwrap();
}

fn save_session_key(session_key: &str) {
    println!("Saving a session key: {}", session_key);

    File::create(Path::new(SESSION_KEY_FILE))
            .map(|mut file| file.write_all(session_key.as_bytes()));
}

fn load_session_key() -> Option<String> {
    println!("Loading a session key");

    let mut session_key = String::new();

    File::open(Path::new(SESSION_KEY_FILE))
            .map(|mut f| f.read_to_string(&mut session_key));

    println!("A session key was loaded: {}", session_key);

    if session_key.is_empty() {
        None
    } else {
        Some(session_key)
    }
}

fn make_req_body_urlencoded(params: &Vec<(&str, &str)>) -> Result<String, serde_urlencoded::ser::Error> {
    let sig = format!("{}", build_signature(&params));
    let mut params_copy = params.clone();
    params_copy.push(("api_sig", &sig));

    serde_urlencoded::to_string(params_copy)
}

fn make_req_body(params: &Vec<(&str, &str)>) -> String {
    let mut json = String::new();
    let mut first_run = true;
    for &(k, v) in params {
        if first_run {
            json.push_str(&format!("{{ \"{}\": \"{}\"", k, v));
            first_run = false;
            continue;
        }
        json.push_str(&format!(", \"{}\": \"{}\"", k, v));
    }
    json.push_str(&format!(", \"api_sig\": \"{}\"", build_signature(&params)));
    json.push_str(&format!("{}", " }"));

    json
}

fn build_url(params: &Vec<(&str, &str)>) -> String {
    let mut params_slice = Vec::new();
    for &(k, v) in params {
        params_slice.push(format!("{}={}", k, v));
    }
    params_slice.push(format!("api_sig={}", build_signature(&params)));
    params_slice.push("format=json".to_string());

    let mut url = "http://ws.audioscrobbler.com/2.0/?".to_string();
    url.push_str(&params_slice.join("&"));

    url.to_string()
}

fn build_signature(params: &Vec<(&str, &str)>) -> String {
    let mut sorted_params = params.clone();
    sorted_params.sort_by_key(|a| a.0);

    let mut sig = String::new();
    for (k, v) in sorted_params {
        sig.push_str((k.to_string() + v).as_str());
    }
    sig.push_str("<your_shared_secret>");
    let mut sig_hash = Md5::new();
    sig_hash.input(sig.as_bytes());

    sig_hash.result_str()
}

fn make_track_params(index: u16, track: &TrackInfo) -> Vec<(String, String)> {
    let mut params = Vec::new();

    params.push((format!("artist[{}]", index), track.artist.to_owned()));
    params.push((format!("track[{}]", index), track.track_name.to_owned()));
    params.push((format!("timestamp[{}]", index), track.timestamp.to_string()));

    if let Some(ref album) = track.album {
        params.push((format!("album[{}]", index), album.to_string()));
    }

    if let Some(track_num) = track.track_number {
        params.push((format!("trackNumber[{}]", index), track_num.to_string()));
    }

    if let Some(duration) = track.duration {
        params.push((format!("duration[{}]", index), duration.to_string()));
    }

    params
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_build_url() {
        let mut params = Vec::new();
        params.push(("lol", "xd"));
        params.push(("lol2", "xd2"));
        let url = build_url(&params);
        println!("{}", &url);
        assert!(url.eq("http://ws.audioscrobbler.com/2.0/?lol=xd&lol2=xd2&api_sig=<your_api_sig>&format=json"));
    }

    #[test]
    #[ignore]
    fn test_track_scrobble_url() {
        let mut tracks = Vec::new();
        tracks.push(TrackInfo { artist: "Rush".to_string(), track_name: "Tom Sawyer".to_string(), timestamp: 223223,
                                duration: Some(12345), album: Some("None".to_string()), track_number: None, status: None, uid: None });
        let track_params = make_track_params(0, &tracks[0]);

        let mut params = Vec::new();
        //params.push(("method", "track.scrobble"));
        for &(ref k, ref v) in &track_params {
            params.push((k.as_str(), v.as_str()));
        }

        println!("{}", build_url(&params));
    }

    #[test]
    #[ignore]
    fn test_get_session() {
        let session_key = get_session(API_KEY);
        if let Some(s) = session_key {
            println!("{}", s);
        }
    }

    #[test]
    #[ignore]
    fn test_save_load_session_key() {
        let session_key = "lolxdkey";
        save_session_key(session_key);
        load_session_key();
        remove_file(SESSION_KEY_FILE);
    }

    #[test]
    // #[ignore]
    fn test_scrobbling() {
        println!("Testing scrobbling");
        let mut tracks = Vec::new();
        tracks.push(TrackInfo { artist: "Rush".to_string(), track_name: "Tom Sawyer".to_string(), timestamp: 228228,
                                duration: Some(12345), album: Some("None".to_string()), track_number: None, status: None, uid: None });
        scrobble_tracks(&mut tracks);
    }

    #[test]
    #[ignore]
    fn test_make_req_body() {
        let mut params = Vec::new();
        //params.push(("method", "track.scrobble"));

        println!("Testing make_req_body \n {}", make_req_body(&params));
    }
}

