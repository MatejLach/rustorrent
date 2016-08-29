use std::net::IpAddr;
use convert::TryFrom;
use metainfo::SHA1Hash20b;
use bencode::{BDict, BString, BInt, BList, DecodeError, DecodeErrorKind};
use std::str::FromStr;

pub struct TrackerReq {
    pub info_hash: SHA1Hash20b,
    pub peer_id: SHA1Hash20b,
    pub port: u32,
    pub uploaded: u64,
    pub left: u64,
    pub compact: bool,
    pub no_peer_id: bool,
    pub event: TrackerEvent,
    pub ip: Option<IpAddr>,
    pub numwant: Option<u32>,
    pub key: Option<String>,
    pub trackerid: Option<String>,
}

fn missing_field(fld: &str) -> DecodeError {
    DecodeError {
        position: None,
        kind: DecodeErrorKind::MissingField(fld.to_string()),
    }
}

impl TryFrom<BDict> for TrackerReq {
    type Err = DecodeError;
    fn try_from(dict: BDict) -> Result<Self, Self::Err> {
        // let info_hash: BString = try!(dict.get_copy("info hash").ok_or(missing_field("info_hash")));
        // let peer_id: BString = try!(dict.get_copy("peer id").ok_or(missing_field("peer id")));
        // let port: BInt = try!(dict.get_copy("port").ok_or(missing_field("port")));
        // let uploaded: BInt = try!(dict.get_copy("uploaded").ok_or(missing_field("uploaded")));
        // let left: BInt = try!(dict.get_copy("uploaded").ok_or(missing_field("uploaded")));
        unimplemented!();
    }
}

impl TryFrom<BDict> for TrackerResp {
    type Err = DecodeError;
    fn try_from(dict: BDict) -> Result<Self, Self::Err> {
        //required fields
        let interval: BInt = try!(dict.get_copy("interval").ok_or(missing_field("interval")));
        let tracker_id: String = try!(dict.get_copy("tracker id")
            .ok_or(missing_field("tracker id")));
        let complete: BInt = try!(dict.get_copy("complete").ok_or(missing_field("complete")));
        
        //optional fields
        let failure_reason: Option<String> = dict.get_copy("failure reason");
        let warning_message: Option<String> = dict.get_copy("warning message");
        let min_interval: Option<BInt> = dict.get_copy("min interval");
        
        //parse the peer list
        let peers_blist: Vec<BDict> = try!(dict.get_copy("peers").ok_or(missing_field("peers")));
        let mut peers_list = Vec::new();
        for peer in peers_blist {
            let peer_id: String = try!(peer.get_copy("peer id").ok_or(missing_field("peer id")));
            let peer_ip: String = try!(peer.get_copy("ip").ok_or(missing_field("ip")));
            let peer_port: BInt = try!(peer.get_copy("port").ok_or(missing_field("port")));
            let ip = try!(IpAddr::from_str(&peer_ip).map_err(|_| missing_field("ip")));

            peers_list.push(Peer {
                peer_id: peer_id,
                ip: ip,
                port: peer_port.to_i64() as u32,
            });
        }

        //piece it together
        Ok(TrackerResp {
            failure_reason: failure_reason,
            warning_message: warning_message,
            interval: interval.to_i64() as u32,
            min_interval: min_interval.map(|i| i.to_i64() as u32),
            tracker_id: tracker_id,
            complete: complete.to_i64() as u32,
            peers: peers_list,
        })
    }
}

pub enum TrackerEvent {
    Started,
    Stopped,
    Completed,
}

pub struct TrackerResp {
    pub failure_reason: Option<String>,
    pub warning_message: Option<String>,
    pub interval: u32,
    pub min_interval: Option<u32>,
    pub tracker_id: String,
    pub complete: u32,
    pub peers: Vec<Peer>,
}

pub struct Peer {
    pub peer_id: String,
    pub ip: IpAddr,
    pub port: u32,
}
