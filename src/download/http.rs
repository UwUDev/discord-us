use std::{
    io::{
        Error, Result,
    },
    ops::Range,
};

use crate::{
    download::Download,
    utils::read::ReadProxy,
};

pub struct HttpDownloader {
    url: String,
    range: Range<u64>,
}

impl HttpDownloader {
    pub fn new(url: String, range: Range<u64>) -> Self {
        Self {
            url,
            range,
        }
    }
}

impl Download<ReadProxy> for HttpDownloader {
    fn download(&self) -> Result<ReadProxy> {
        let res = ureq::request("GET", &self.url)
            .set("User-Agent", "Mozilla/5.0")
            .set("Range", &format!("bytes={}-{}", self.range.start, self.range.end))
            .call().map_err(|e| Error::new(std::io::ErrorKind::Other, e))?;

        let reader = res.into_reader().into();

        Ok(reader)
    }
}