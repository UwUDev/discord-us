use std::io::{Error, Read};
use std::ops::Range;
use std::time::Duration;

use rand::distributions::Alphanumeric;
use rand::Rng;
use serde_json::json;
use ureq::{Agent, AgentBuilder};

use crate::signal::AddSignaler;
use crate::signal::progress::{ProgressSignal, ProgressSignalTrait};
use crate::upload::{Uploader, UploaderCoolDownResponse, UploaderMaxSize};
use crate::upload::account::AccountCredentials;
use crate::utils::limit::CoolDownMs;
use crate::utils::read::StaticStream;

const MAX_WEBHOOK_SIZE: u64 = 24 * 1024 * 1024;

#[derive(Clone)]
pub struct WebhookUploader {
    credentials: AccountCredentials,
    agent: Agent,
    include_token: bool,
}

impl WebhookUploader {
    pub fn new(credentials: AccountCredentials) -> Self {
        let agent = AgentBuilder::new()
            .timeout_read(Duration::from_secs(60))
            .timeout_write(Duration::from_secs(60 * 60))
            .build();

        Self { credentials, agent, include_token: false }
    }

    pub fn include_token(&mut self, include: bool) {
        self.include_token = include
    }

    fn generate_boundary() -> String {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(24)
            .map(char::from)
            .collect()
    }
}

impl UploaderMaxSize for WebhookUploader {
    fn get_max_size(&self) -> u64 {
        MAX_WEBHOOK_SIZE
    }
}

impl CoolDownMs for WebhookUploader {
    fn get_cool_down() -> (f64, u32) {
        (0.0, 5)
    }
}

impl<R: Read, S: AddSignaler<Range<u64>>> Uploader<String, R, S> for WebhookUploader {
    fn do_upload(&mut self, reader: R, size: u64, signal: &mut ProgressSignal<S>) -> Result<UploaderCoolDownResponse<String>, Error> {
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
        )).chain(crate::upload::bot::FormDataStream {
            reader,
            signal,
            read: 0,
            size,
        }).chain(StaticStream::from(
            format!("\r\n--{}--\r\n", boundary.clone()).into()
        ));

        let url = format!("https://discord.com/api/webhooks/{}/{}?wait=true", self.credentials.channel_id, self.credentials.access_token);

        let response = self.agent.post(&url)
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

        #[cfg(test)] println!("Remaining: {} | Reset after: {:?}", remaining, reset_after);


        let data = response.into_json::<serde_json::Value>()?;
        let message_id = data["id"].as_str().unwrap();
        let file_url = data["attachments"][0]["url"]
            .as_str()
            .ok_or_else(|| Error::new(std::io::ErrorKind::Other, "upload_url not found"))?;

        if !signal.is_running() {
            return Err(Error::new(std::io::ErrorKind::Interrupted, "Upload interrupted"));
        }

        // if webhook token is needed format the url accordingly
        let url = if self.include_token {
            let encoded = url::form_urlencoded::byte_serialize(file_url.as_bytes()).collect::<String>();
            format!("webhook://{}?webhook_id={}&token={}&url={}",
                    message_id,
                    self.credentials.channel_id,
                    self.credentials.access_token,
                    encoded)
        } else {
            file_url.to_string()
        };

        Ok(UploaderCoolDownResponse::CoolDown(url, (reset_after * 1000.0) as u64, remaining))
    }
}

#[cfg(test)]
mod test {
    use std::fs::File;
    use std::ops::Range;

    use crate::{signal::{
        progress::{
            ProgressSignal,
            ProgressSignalAccessor,
        },
        StoredSignal,
    }, upload::{
        account::{
            AccountCredentials,
            AccountSubscription,
        },
        Uploader,
        webhook::WebhookUploader,
    },
    };
    use crate::signal::StaticSignal;
    use crate::utils::safe::SafeAccessor;

    #[test]
    pub fn test_webhook() {
        let mut uploader = WebhookUploader::new(AccountCredentials {
            channel_id: 0,
            access_token: "//".to_string(),
            subscription: AccountSubscription::Free,
        });
        uploader.include_token(true);

        let mut signal = ProgressSignal::<StoredSignal<Vec<Range<u64>>>>::new();

        let mut file = File::open("test.mp4").unwrap();
        let len = file.metadata().unwrap().len();

        let start = std::time::Instant::now();

        let url = uploader.do_upload(&mut file, len, &mut signal).unwrap();

        let mut signal = signal.get_progression().access();
        signal.retrim_ranges();
        #[cfg(
            test
        )] println!("Uploaded | signal = {:?} | elapsed {:?} | url = {}", signal.get_signal_data(), start.elapsed(), url.unwrap());
    }
}