extern crate rustorrent;
extern crate sha1;
extern crate mio;
extern crate hyper;
#[macro_use]
extern crate log;

use rustorrent::init;
use rustorrent::bencode::decode::{belement_decode, DecodeResult};
use rustorrent::bencode::BDict;
use rustorrent::metainfo::{MetaInfo, SHA1Hash20b};
use rustorrent::wire::{Protocol, ChanMsg};
use rustorrent::convert::TryFrom;
use rustorrent::bencode::Bencode;
use rustorrent::bencode::DecodeError;
use rustorrent::metainfo::MetaInfoError;
use rustorrent::tracker::HttpTrackerHandler;
use rustorrent::tracker::{TrackerResp, TrackerError, TrackerEvent};

use std::env;
use std::fs::File;
use std::io;
use std::time::Duration;
use std::time::SystemTime;
use std::thread::{sleep, spawn};
use std::io::Read;
use std::thread;
use std::thread::JoinHandle;

use hyper::Url;
use rustorrent::tracker::http::TrackerHandler;
use rustorrent::tracker::TrackerReq;

use mio::channel::{Sender, Receiver};
use sha1::Sha1;

const DEFAULT_PORT: u32 = 12001;
const DEFAULT_PEER_ID: &'static str = "-RT0001-048230984201";

pub fn main() {
    init();
    let mut args = env::args();
    if let Some(path_string) = args.nth(1) {
        info!("Starting up");
        let result = _begin_with_path(path_string);
    } else {
        _usage();
    }
}

type SuccessType = ();

#[derive(Debug)]
enum FatalError {
    IOError(io::Error),
    DecodeError(DecodeError),
    MetaInfoError(MetaInfoError),
}

fn _begin_with_path(path_string: String) -> Result<SuccessType, FatalError> {
    let mut bytes: Vec<u8> = Vec::new();
    let belement: Bencode;
    let bdict: BDict;
    let metainfo: MetaInfo;

    // read the file and change the result type if fail
    let mut file_open_result = File::open(&path_string);
    if let Ok(mut read) = file_open_result {
        let read_result = read.read_to_end(&mut bytes);
        if !read_result.is_ok() {
            return Ok(());
        }
    } else {
        return Err(FatalError::IOError(file_open_result.err().unwrap()));
    }

    // compute the very important metainfo hash
    let mut sha1 = Sha1::new();
    sha1.update(&bytes);
    let hash = sha1.digest().bytes();

    // parse into a bencoded structure
    let parse_result = belement_decode(&bytes);
    if let Ok(DecodeResult(Bencode::BDict(dict), offset)) = parse_result {
        bdict = dict;
    } else {
        return Err(FatalError::DecodeError(parse_result.err().unwrap()));
    }

    let metainfo_result = MetaInfo::try_from(bdict);
    if let Ok(metainfo) = metainfo_result {
        let mut hash_array = Vec::new();
        for &b in hash.iter() {
            hash_array.push(b);
        }
        _begin_protocol_session(&metainfo, hash_array);
    } else {
        return Err(FatalError::MetaInfoError(metainfo_result.err().unwrap()));
    }

    return Ok(());
}


fn _begin_protocol_session(info: &MetaInfo, hash: SHA1Hash20b) {
    let real_hash = match info.info.original {
        Some(ref original_dict) => original_dict.hash(),
        None => panic!("No freaking hash!"),
    };

    match Protocol::new(info, real_hash.clone(), DEFAULT_PEER_ID) {
        (protocol, sender, receiver) => {
            let pwp = _start_peer_wire_protocol_thread(protocol);
            _start_tracker(&hash,
                           info,
                           &DEFAULT_PEER_ID.to_string().into_bytes(),
                           sender,
                           receiver);
        }
    }
}

fn _start_peer_wire_protocol_thread(mut protocol: Protocol) -> JoinHandle<()> {
    thread::spawn(move || protocol.run())
}

fn _start_tracker(hash: &SHA1Hash20b,
                  info: &MetaInfo,
                  peer_id: &SHA1Hash20b,
                  sender: Sender<ChanMsg>,
                  recv: Receiver<ChanMsg>) {
    const DEFAULT_TRACKER_INTERVAL_SECONDS: u64 = 10;
    let SLEEP_DURATION: Duration = Duration::from_millis(100);
    let mut last_request_time: SystemTime = SystemTime::now();
    let mut result_response = _get_tracker_response(hash, info, peer_id);
    let mut interval: u64 = DEFAULT_TRACKER_INTERVAL_SECONDS;

    loop {
        let response_result = _get_tracker_response(hash, info, peer_id);
        match &result_response {
            &Ok(ref r) => {
                info!("Querying tracker...");
                last_request_time = SystemTime::now();
                for peer in r.peers.iter() {
                    let msg = ChanMsg::NewPeer(peer.ip, peer.port);
                    sender.send(msg);
                }
                info!("Tracker has {} peers", r.peers.len());
                if let Some(i) = r.interval {
                    interval = i as u64;
                }
            }
            &Err(Some(ref e)) => {
                info!("Querying tracker failed: {}", e);
                continue;
            }
            &Err(_) => {
                info!("Unknown error");
                continue;
            }

        };

        interval = 10000;

        info!("Interval between requests is {} second(s)", interval);
        thread::sleep(Duration::from_millis(interval * 1000));
    }

    // TODO Implement this - goal is that it queries that tracker at a defined period
    // Sends list of peers to pwp using Sender
}

fn _get_tracker_response(hash: &SHA1Hash20b,
                         info: &MetaInfo,
                         peer_id: &SHA1Hash20b)
                         -> Result<TrackerResp, Option<TrackerError>> {
    let url_result = Url::parse(&info.announce);
    if !url_result.is_ok() {
        return Err(None); //TODO Signal some kind of parse error
    }
    let url = url_result.unwrap();
    let mut handler = HttpTrackerHandler::new(url);
    let request: TrackerReq = _get_request_obj(hash, peer_id, info);
    handler.request(&request).map_err(|e| Some(e))
}

fn _get_request_obj(hash: &SHA1Hash20b,
                    peer_id: &SHA1Hash20b,
                    info: &MetaInfo)
                    -> TrackerReq {
    let mut info_hash = Vec::new();
    info_hash.resize(20, 0);

    match info.info.original {
        Some(ref original_dict) => info_hash = original_dict.hash(),
        None => info!("No hash!"),
    };

    TrackerReq {
        info_hash: info_hash,
        peer_id: peer_id.clone(),
        port: DEFAULT_PORT,
        uploaded: 0,
        downloaded: 0,
        left: info.info.pieces.len() as u64,
        compact: false,
        no_peer_id: false,
        event: TrackerEvent::Started,
        ip: None,
        numwant: None,
        key: None,
        trackerid: None,
    }
}

fn _usage() {
    match env::current_exe() {
        Ok(path) => info!("Usage: {} torrent_file", path.display()),
        _ => info!("Invalid arguments. Format is: torrent_file"),
    }
}
