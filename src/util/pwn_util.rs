use std::{vec::Vec, str};
use rand::{thread_rng, Rng, distributions::Alphanumeric};
// use openssl::{sha::sha256, hash::{hash, MessageDigest}};
use tokio::{sync::mpsc::{channel, Sender, Receiver}};
use anyhow::Result;
use reqwest;
use serde::{Deserialize};
use md5;
use hex;
struct Pow {
    salt: Vec<u8>,
    hard_bit: usize, 
}

// impl Pow {
//     pub fn new(hard: usize) -> Pow {
//         Pow{salt: Pow::random_salt(20), hard_bit: hard}
//     }

//     pub fn pow_message(self: &Self) -> String {
//         format!("sha256(\"{}\"+\"?\") starts with {}bits of zero: ", str::from_utf8(&self.salt).unwrap(), self.hard_bit)
//     }

//     pub fn challenge(self: &Self,mut challenge_data: Vec<u8>) -> bool {
//         let mut s = self.salt.clone();
//         s.append(&mut challenge_data);
//         let sha256_mk = sha256(&s);
//         let bit = self.hard_bit;
//         for i in 0..=bit/8 {
//             if sha256_mk[i] != 0 {
//                 return false;
//             }
//         }

//         if sha256_mk[(bit/8)] >> (8 - bit%8) == 0 {
//             return true;
//         }else {
//             return false;
//         }
//     }

//     fn random_salt(size: usize) -> Vec<u8> {
//         thread_rng().sample_iter(&Alphanumeric).take(size).map(u8::from).collect()
//     }

// }


#[derive(Deserialize, Debug)]
struct TeamInfo {
    err: Option<String>,
    msg: String,
    data: Vec<TeamData>,
}

#[derive(Deserialize, Debug)]
struct TeamData {
    _id: String,
    hash: String,
    create_time: f64,
}

//CCqkSoOeRrSw8rIWKheAiQ==
pub async fn token_checker(token: String) -> Result<bool> {

    // return Ok(true);
    // let md_token =&*hash(MessageDigest::md5(), trim_token.as_bytes())?;
    let md_token = md5::compute(token.as_bytes());
    let hex_token = hex::encode(md_token.0);

    let client = reqwest::Client::new();
    let resp = client.get("https://realworldctf.com/api/team/info").header("token", "c5lrlOLxazaSEI").send().await?;
    println!("code {}", resp.status());
    let res = resp.json::<TeamInfo>().await?;
    for f in res.data.iter() {
        if f.hash == hex_token {
            return Ok(true);
        }
    }
    Ok(false)
}


pub struct ChallQueue {
    working: usize,
    waited: Vec<Sender<u64>>,
    max_working: usize,
}

impl ChallQueue{
    pub fn new(max: usize) -> ChallQueue {
        ChallQueue{working:0, waited: Vec::new(), max_working: max}
    }

    pub async fn queue_in(self:&mut Self) -> Option<Receiver<u64>> {
        if self.working < self.max_working {
            self.working += 1;
            return None;
        }else {
            let (tx, rx) = channel(100);
            tx.send(self.waited.len() as u64 +1).await;
            self.waited.push(tx);
            Some(rx)
        }
    }

    pub async fn queue_out(self:&mut Self) {
        if self.waited.is_empty() {
            self.working -= 1;
        }else {
            let tx = self.waited.remove(0);
            tx.send(0).await;
            let mut i = 1;
            for t in self.waited.iter() {
                t.send(i).await;
                i += 1;
            }
        }
    }

}