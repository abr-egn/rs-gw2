use std::time::{Duration, Instant};

use reqwest::{self, StatusCode};
use serde::de::DeserializeOwned;

use crate::error::{Result};

pub struct Client {
    reqw: reqwest::blocking::Client,
    last: Instant,
}

static KEY: &str = "9E827119-71EE-774C-88AE-B4D1D4B30868205CD293-9A69-415C-A3E3-4CB1E184722C";

impl Client {
    pub fn new() -> Self {
        Client {
            reqw: reqwest::blocking::Client::new(),
            last: Instant::now(),
        }
    }

    pub fn fetch<Out>(
        &mut self,
        auth: bool,
        path: &str,
    ) -> Result<Out>
    where
        Out: DeserializeOwned,
    {
        let since = Instant::now().duration_since(self.last);
        let tick = Duration::from_secs_f32(0.1);
        if since < tick {
            std::thread::sleep(tick - since);
        }
        self.last = Instant::now();

        let mut req = self.reqw.get(&format!("https://api.guildwars2.com/v2/{}", path))
            .query(&[("v", "latest")]);
        if auth {
            req = req.query(&[("access_token", KEY)]);
        }
        let req = req.build()?;
        let mut res = self.reqw.execute(req.try_clone().unwrap())?;
        if res.status() == StatusCode::TOO_MANY_REQUESTS {
            println!("\t429 sleep");
            std::thread::sleep(tick + tick);
            self.last = Instant::now();
            res = self.reqw.execute(req)?;
        }
        match res.status() {
            StatusCode::OK => (),
            StatusCode::PARTIAL_CONTENT => /*println!("[Partial {:?}]", res)*/ (),
            _ => failed!("{:?}", res),
        }

        Ok(res.json()?)
    }
}