/// The downloading of webhook is somehow complicated
/// the webhook url is webhook://<webhook_id>?token=<token>&url=<url>
/// the discord url can contain an expiration time, so we need to check if the url is expired
/// if the url is missing or expired, we need to regenerate it and this part is tricky

/// The ideal candidate would have to have a separated "service" which is responsible for token sharing
/// and Rate-Limiting. Ideally a global "webhook" resolver should store current rate limit status for each webhook_id
/// Using a global resolver instead of local, scoped resolver per download requests allow to better manage concurrency

use std::collections::HashMap;
use std::io::Error;
use std::thread::sleep;
use std::time::{Duration, Instant};

use ureq::{Agent, AgentBuilder};
use url::Url;

use crate::utils::limit::{CoolDown, CoolDownMs};
use crate::utils::safe::{Safe, SafeAccessor};

/// The webhook resolver
#[derive(Clone)]
pub struct WebhookResolver {
    webhooks: Safe<HashMap<u64, Webhook>>,
    agent: Agent,
}

#[derive(Clone)]
pub struct Webhook {
    id: u64,
    token: String,
    agent: Agent,

    cool_down: CoolDown,
}

pub struct WebhookResponse {
    pub url: String,
    pub remaining: u32,
    pub reset: Duration,
}

impl CoolDownMs for Webhook {
    fn get_cool_down() -> (f64, u32) {
        (0.0, 5)
    }
}

/// Check whether a webhook url is valid or not by comparing the expiration time to the current time
/// return true if the url is valid, false otherwise
pub fn is_valid_webhook(url: &Url) -> bool {
    match url.query_pairs().find(|(k, _)| k == "ex") {
        Some((_, exp)) => {
            let exp = u64::from_str_radix(&exp, 16);
            match exp {
                Ok(exp) => exp > std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                Err(_) => false
            }
        }
        None => false
    }
}

impl Default for WebhookResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl WebhookResolver {
    pub fn new() -> Self {
        let agent = AgentBuilder::new()
            .timeout_read(Duration::from_secs(60))
            .timeout_write(Duration::from_secs(60 * 60))
            .build();

        Self {
            webhooks: Safe::wrap(HashMap::new()),
            agent,
        }
    }

    pub fn resolve(&self, url: Url) -> std::io::Result<(String, String)> {
        // unlock hashmap
        let id = url.query_pairs().find(|(k, _)| k == "webhook_id").unwrap().1.parse::<u64>().unwrap();
        let token = url.query_pairs().find(|(k, _)| k == "token").unwrap().1;

        let mut map = self.webhooks.access();

        let webhook = match map.get_mut(&id) {
            Some(webhook) => webhook,
            None => {
                let webhook = Webhook {
                    id,
                    token: token.to_string(),
                    agent: self.agent.clone(),
                    cool_down: Webhook::create_cooldown_wait(),
                };
                map.insert(id, webhook);
                map.get_mut(&id).unwrap()
            }
        };

        if !webhook.cool_down.can_accept_more() {
            sleep(Duration::from_millis(50)); // retry after 50ms
            drop(map);
            return self.resolve(url);
        }

        webhook.cool_down.start_work();

        // clone and drop lock
        let mut w = webhook.clone();
        drop(map);

        let message_id = url.domain().unwrap();

        let resp = w.resolve(message_id)?;
        if resp.remaining == 0 {
            w.cool_down.set_duration(resp.reset);
            w.cool_down.set_max_concurrency(w.cool_down.get_concurrency().max(1))
        }
        w.cool_down.end_work(Instant::now());

        let encoded = url::form_urlencoded::byte_serialize(resp.url.as_bytes()).collect::<String>();

        let w_url = format!("webhook://{}?webhook_id={}&token={}&url={}",
                            message_id,
                            id,
                            token,
                            encoded);

        Ok((resp.url, w_url))
    }
}


impl Webhook {
    pub fn resolve(&self, message_id: &str) -> std::io::Result<WebhookResponse> {
        // message ID is the third segment of the path
        // let discord = url.query_pairs().find(|(k, _)| k == "url").unwrap().1;

        let url = Url::parse(&format!("https://discord.com/api/webhooks/{}/{}/messages/{}", self.id, self.token, message_id)).unwrap();

        let resp = self.agent.get(url.as_str()).call()
            .map_err(|e| Error::new(std::io::ErrorKind::Other, e))?;

        if resp.status() != 200 {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "Invalid response"));
        }

        let remaining = resp.header("x-ratelimit-remaining").unwrap_or("1").parse::<u32>().unwrap();
        let reset = resp.header("x-ratelimit-reset-after").unwrap_or("0").parse::<f64>().unwrap();

        let data = resp.into_json::<serde_json::Value>()?;

        let attachment = data.get("attachments").unwrap().get(0).ok_or_else(|| Error::new(std::io::ErrorKind::Other, "Attachment not found"))?;

        let url = attachment.get("url").ok_or_else(|| Error::new(std::io::ErrorKind::Other, "URL not found"))?;

        //#[cfg(test)] println!("Resolved URL: {}", url);


        Ok(WebhookResponse {
            url: url.as_str().unwrap().to_string(),
            remaining,
            reset: Duration::from_secs_f64(reset),
        })
    }
}