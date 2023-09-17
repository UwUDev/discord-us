use std::{
    ops::Range,
    io::{Read, Error},
    time::{Duration},
};
use rand::{distributions::Alphanumeric, Rng};
use serde_json::json;
use crate::{
    upload::{
        Uploader,
        UploaderMaxSize,
        UploaderCoolDownResponse,
        account::{
            AccountCredentials,
        },
    },
    signal::{
        AddSignaler,
        progress::{ProgressSignal, ProgressSignalTrait},
    },
    utils::{
        read::{
            StaticStream,
        },
        limit::{
            CoolDownMs,
        },
    },
    Size,
};

use ureq::{Agent, AgentBuilder};

#[derive(Clone)]
pub struct BotUploader {
    credentials: AccountCredentials,

    agent: Agent,
}

impl UploaderMaxSize for BotUploader {
    fn get_max_size(&self) -> u64 {
        self.credentials.subscription.get_max_upload_size() as u64
    }
}

impl CoolDownMs for BotUploader {
    fn get_cool_down(&self) -> (f64, u32) {
        return (0.0, 5);
    }
}

impl BotUploader {
    pub fn new(credentials: AccountCredentials) -> Self {
        let agent = AgentBuilder::new()
            .timeout_read(Duration::from_secs(60))
            .timeout_write(Duration::from_secs(60 * 60))
            .build();

        Self { credentials, agent }
    }

    fn generate_boundary() -> String {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(24)
            .map(char::from)
            .collect()
    }
}

impl<R: Read, S: AddSignaler<Range<u64>>> Uploader<String, R, S> for BotUploader {
    fn do_upload(&mut self, reader: R, size: u64, signal: ProgressSignal<S>) -> Result<UploaderCoolDownResponse<String>, Error> {
        let boundary = Self::generate_boundary();

        let payload_json = json!({
            "attachments": [{
                "id": 0,
                "description": "File",
                "filename": "data.bin",
            }]
        });

        let mut body = StaticStream::from(
            format!("--{}\r\nContent-Disposition: form-data; name=\"payload_json\"\r\nContent-Type: application/json\r\n\r\n", boundary.clone()).into()
        ).chain(
            StaticStream::from(
                serde_json::to_string(&payload_json).unwrap().into()
            )
        ).chain(StaticStream::from(
            format!("\r\n--{}\r\nContent-Disposition: form-data; name=\"files[0]\"; filename=\"data.bin\"\r\n\r\n", boundary.clone()).into()
        )).chain(FormDataStream {
            reader,
            signal: signal,
            read: 0,
            size,
        }).chain(StaticStream::from(
            format!("\r\n--{}--\r\n", boundary.clone()).into()
        ));

        let url = format!("https://discord.com/api/v9/channels/{}/messages", self.credentials.channel_id);

        let response = self.agent.post(&url)
            .set("Authorization", format!("Bot {}", &self.credentials.access_token).as_str())
            .set("Content-Type", format!("multipart/form-data; boundary={}", boundary).as_str())
            .send(&mut body)
            .map_err(|e| Error::new(std::io::ErrorKind::Other, e))?;

        let remaining = match response.header("x-ratelimit-remaining") {
            Some(x) => x.parse::<u32>().unwrap(),
            _ => 1,
        };

        let reset_after = match response.header("x-ratelimit-reset-after") {
            Some(x) => x.parse::<f64>().unwrap(),
            _ => 0.0
        };

            println!("Remaining: {} | Reset after: {:?}", remaining, reset_after);


            let data = response.into_json::<serde_json::Value>() ?;

            let file_url = data["attachments"][0]["url"]
                .as_str()
                .ok_or_else(|| Error::new(std::io::ErrorKind::Other, "upload_url not found"))?;

            Ok(UploaderCoolDownResponse::CoolDown(file_url.into(), (reset_after * 1000.0) as u64, remaining))
        }
    }

    struct FormDataStream<R: Read, S: AddSignaler<Range<u64>>> {
        reader: R,
        signal: ProgressSignal<S>,
        read: u64,
        size: u64,
    }

    impl<R: Read, S: AddSignaler<Range<u64>>> Size for FormDataStream<R, S> {
        fn get_size(&self) -> u64 {
            self.size
        }
    }

    impl<R: Read, S: AddSignaler<Range<u64>>> Read for FormDataStream<R, S> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if !self.signal.is_running() {
                return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "Upload interrupted"));
            }

            let read = self.reader.read(buf)?;

            self.signal.add_signal(self.read..self.read + read as u64);
            self.read += read as u64;
            Ok(read)
        }
    }

    #[cfg(test)]
    mod test {
        use crate::{upload::{
            account::{
                AccountCredentials,
                AccountSubscription,
            },
            bot::{
                BotUploader,
            },
            Uploader,
        }, signal::{
            StoredSignal,
            progress::{
                ProgressSignal,
                ProgressSignalAccessor,
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
            let mut uploader = BotUploader::new(AccountCredentials {
                channel_id: 0,
                access_token: "//".to_string(),
                subscription: AccountSubscription::Free,
            });

            let signal = ProgressSignal::<StoredSignal<Vec<Range<u64>>>>::new();

            let mut file = File::open("test.mp4").unwrap();
            let len = file.metadata().unwrap().len();

            let start = std::time::Instant::now();

            let url = uploader.do_upload(&mut file, len, signal.clone_with_offset(0)).unwrap();

            let mut signal = signal.get_progression().access();
            signal.retrim_ranges();
            println!("Uploaded | signal = {:?} | elapsed {:?} | url = {}", signal.get_signal_data(), start.elapsed(), url.unwrap());
        }
    }