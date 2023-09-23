use std::{
    ops::Range,
    io::{Read, Error},
    time::{Duration},
};
use crate::{
    upload::{
        Uploader,
        UploaderMaxSize,
        UploaderCoolDownResponse,
    },
    signal::{
        AddSignaler,
        progress::{ProgressSignal,ProgressSignalTrait},
    },
};

use ureq::{Agent, AgentBuilder};

#[derive(Clone)]
pub enum AccountSubscription {
    Free,
    Basic,
    Classic,
    Boost,
}

impl AccountSubscription {
    pub fn get_max_upload_size(&self) -> usize {
        match *self {
            Self::Free => 25 * 1024 * 1024,
            Self::Basic => 50 * 1024 * 1024,
            Self::Classic => 100 * 1024 * 1024,
            Self::Boost => 500 * 1024 * 1024,
        }
    }
}

#[derive(Clone)]
pub struct AccountCredentials {
    pub(crate) channel_id: u64,
    pub(crate) access_token: String,
    pub(crate) subscription: AccountSubscription,
}

#[derive(Clone)]
pub struct AccountUploader {
    credentials: AccountCredentials,

    agent: Agent,
}

impl UploaderMaxSize for AccountUploader {
    fn get_max_size(&self) -> u64 {
        self.credentials.subscription.get_max_upload_size() as u64
    }
}

const DISCORD_HEADERS: &'static [(&'static str, &'static str)] = &[
    ("Content-Type", "application/json"),
    ("X-Super-Properties", "eyJvcyI6IkFuZHJvaWQiLCJicm93c2VyIjoiRGlzY29yZCBBbmRyb2lkIiwiZGV2aWNlIjoiYmx1ZWpheSIsInN5c3RlbV9sb2NhbGUiOiJmci1GUiIsImNsaWVudF92ZXJzaW9uIjoiMTkyLjEzIC0gcm4iLCJyZWxlYXNlX2NoYW5uZWwiOiJnb29nbGVSZWxlYXNlIiwiZGV2aWNlX3ZlbmRvcl9pZCI6IjhkZGU4M2IzLTUzOGEtNDJkMi04MzExLTM1YmFlY2M2YmJiOCIsImJyb3dzZXJfdXNlcl9hZ2VudCI6IiIsImJyb3dzZXJfdmVyc2lvbiI6IiIsIm9zX3ZlcnNpb24iOiIzMyIsImNsaWVudF9idWlsZF9udW1iZXIiOjE5MjAxMzAwMTEzNzczLCJjbGllbnRfZXZlbnRfc291cmNlIjpudWxsLCJkZXNpZ25faWQiOjB9"),
    ("Accept-Language", "fr-FR"),
    ("X-Discord-Locale", "fr"),
    ("X-Discord-Timezone", "Europe/Paris"),
    ("X-Debug-Options", "bugReporterEnabled"),
    ("User-Agent", "Discord-Android/192013;RNA"),
    ("Host", "discord.com"),
    ("Connection", "Keep-Alive"),
    ("Accept-Encoding", "gzip"),
];

const FILENAME: &'static str = "data.bin";

impl AccountUploader {
    pub fn new(credentials: AccountCredentials) -> Self {
        let agent = AgentBuilder::new()
            .timeout_read(Duration::from_secs(60))
            .timeout_write(Duration::from_secs(60 * 60))
            .build();

        Self {
            credentials,
            agent,
        }
    }

    fn request_attachment_upload_url(&self, size: u64) -> Result<(String, String), Error> {
        let url = format!("https://discord.com/api/v9/channels/{}/attachments", self.credentials.channel_id);

        let mut request = self.agent
            .post(url.as_str())
            .set("Authorization", self.credentials.access_token.as_str());

        for (key, value) in DISCORD_HEADERS {
            request = request.set(key, value);
        }

        let response = request.send_json(ureq::json!({
                "files": [
                    {
                        "filename": FILENAME,
                        "file_size": size,
                        "id": "8"
                    }
                ]
            })).map_err(|e| Error::new(std::io::ErrorKind::Other, e))?
            .into_json::<serde_json::Value>()?;

        let upload_url = response["attachments"][0]["upload_url"]
            .as_str()
            .ok_or_else(|| Error::new(std::io::ErrorKind::Other, "upload_url not found"))?;

        let upload_filename = response["attachments"][0]["upload_filename"]
            .as_str()
            .ok_or_else(|| Error::new(std::io::ErrorKind::Other, "upload_filename not found"))?;

        return Ok((upload_url.to_string(), upload_filename.to_string()));
    }

    fn post_message(&self, upload_filename: &String) -> Result<String, Error> {
        let url = format!("https://discord.com/api/v9/channels/{}/messages", self.credentials.channel_id);

        let mut request = self.agent
            .post(url.as_str())
            .set("Authorization", self.credentials.access_token.as_str());

        for (key, value) in DISCORD_HEADERS {
            request = request.set(key, value);
        }

        let response = request.send_json(ureq::json!({
                "content": "",
                "channel_id": self.credentials.channel_id,
                "type": 0,
                "attachments": [
                    {
                        "id": "0",
                        "filename": FILENAME,
                        "uploaded_filename": upload_filename,
                    }
                ]
            })).map_err(|e| Error::new(std::io::ErrorKind::Other, e))?
            .into_json::<serde_json::Value>()?;

        let file_url = response["attachments"][0]["url"].as_str().unwrap();

        return Ok(file_url.to_string());
    }
}


impl<R: Read, S: AddSignaler<Range<u64>>> Uploader<String, R, S> for AccountUploader {
    fn do_upload(&mut self, reader: R, size: u64, signal: &mut ProgressSignal<S>) -> Result<UploaderCoolDownResponse<String>, Error> {
        let (upload_url, upload_filename) = self.request_attachment_upload_url(size)?;

        struct ReaderWrapper<'a, R: Read, S: AddSignaler<Range<u64>>> {
            reader: R,
            signal: &'a mut ProgressSignal<S>,
            read: u64,
        }

        impl<'a , R: Read, S: AddSignaler<Range<u64>>> Read for ReaderWrapper<'a, R, S> {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                if !self.signal.is_running() {
                    return Err(Error::new(std::io::ErrorKind::Other, "Upload stopped"));
                }

                let read = self.reader.read(buf)?;

                self.signal.add_signal(self.read..(self.read + read as u64));
                self.read += read as u64;

                return Ok(read);
            }
        }

        let _ = self.agent.put(upload_url.as_str())
            .set("accept-encoding", "gzip")
            .set("connection", "Keep-Alive")
            .set("content-length", size.to_string().as_str())
            .set("content-type", "application/x-x509-ca-cert")
            .set("host", "discord-attachments-uploads-prd.storage.googleapis.com")
            .set("user-agent", "Discord-Android/192013;RNA")
            .send(ReaderWrapper {
                reader,
                signal,
                read: 0,
            })
            .map_err(|e| Error::new(std::io::ErrorKind::Other, e))?;


        let result = self.post_message(&upload_filename)?;

        return Ok(UploaderCoolDownResponse::Success(result));
    }
}

#[cfg(test)]
mod test {
    use crate::{
        upload::{
            account::{
                AccountCredentials,
                AccountSubscription,
                AccountUploader,
            },
            Uploader,
        },
        signal::{
            StoredSignal,
            progress::{
                ProgressSignal,
                ProgressSignalAccessor
            },
        },
    };

    use std::{
        ops::{Range}
    };
    use std::fs::File;
    use crate::signal::StaticSignal;
    use crate::utils::safe::SafeAccessor;

    #[test]
    pub fn test_account_uploader() {
        let mut upload = AccountUploader::new(AccountCredentials {
            channel_id: 0,
            access_token: "//".to_string(),
            subscription: AccountSubscription::Free,
        });

        let signal = ProgressSignal::<StoredSignal<Vec<Range<u64>>>>::new();

        let mut file = File::open("test.mp4").unwrap();
        let len = file.metadata().unwrap().len();

        let start = std::time::Instant::now();

        let url = upload.do_upload(&mut file, len, &mut signal.clone_with_offset(0)).unwrap().unwrap();

        let mut signal = signal.get_progression().access();
        signal.retrim_ranges();
        println!("Uploaded | signal = {:?} | elapsed {:?} | url = {}", signal.get_signal_data(), start.elapsed(), url);
    }
}