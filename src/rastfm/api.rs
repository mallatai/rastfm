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
use hyper::client::{Client, Response};

use serde_json::Value;

use crypto::md5::Md5;
use crypto::digest::Digest;

use rastfm::logfile::TrackInfo;

const SESSION_KEY_FILE: &str = "session_key";
const API_KEY: &str = "";
const SHARED_SECRET: &str = "";

type SessionKey = String;

pub struct ApiCredentials {
    username: String,
    password: String,
    api_key: String,
    api_secret: String
}

pub struct AuthorizedApiCredentials {
    username: String,
    password: String,
    api_key: String,
    api_secret: String,
    session_key: SessionKey
}

impl ApiCredentials {
    pub fn authorize(&self) -> Option<AuthorizedApiCredentials> {

        Some ( AuthorizedApiCredentials { username: String::from("lol"), password: String::from("kek"),
                                            api_key: String::from("omg"), api_secret: String::from("lmao"),
                                            session_key: String::from("rofl") } )
    }

    fn get_session(&self) -> Option<SessionKey> {

        Some(String::from("default"))
    }

    fn save_session_key(session_key: &SessionKey) {

    }

    fn load_session_key() -> Option<SessionKey> {

        Some(String::from("default"))
    }
}

impl AuthorizedApiCredentials {

    pub fn scrobble_tracks(tracks: &mut Vec<TrackInfo>) {

    }
}
