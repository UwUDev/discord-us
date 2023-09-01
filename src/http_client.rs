use std::time::Duration;
use reqwest::blocking::{Client, RequestBuilder};

pub fn prepare_discord_request (request: RequestBuilder, token: String) -> RequestBuilder {
    request.header("Authorization", token.clone())
        .header("Content-Type", "application/json")
        .header("X-Super-Properties", "eyJvcyI6IkFuZHJvaWQiLCJicm93c2VyIjoiRGlzY29yZCBBbmRyb2lkIiwiZGV2aWNlIjoiYmx1ZWpheSIsInN5c3RlbV9sb2NhbGUiOiJmci1GUiIsImNsaWVudF92ZXJzaW9uIjoiMTkyLjEzIC0gcm4iLCJyZWxlYXNlX2NoYW5uZWwiOiJnb29nbGVSZWxlYXNlIiwiZGV2aWNlX3ZlbmRvcl9pZCI6IjhkZGU4M2IzLTUzOGEtNDJkMi04MzExLTM1YmFlY2M2YmJiOCIsImJyb3dzZXJfdXNlcl9hZ2VudCI6IiIsImJyb3dzZXJfdmVyc2lvbiI6IiIsIm9zX3ZlcnNpb24iOiIzMyIsImNsaWVudF9idWlsZF9udW1iZXIiOjE5MjAxMzAwMTEzNzczLCJjbGllbnRfZXZlbnRfc291cmNlIjpudWxsLCJkZXNpZ25faWQiOjB9")
        .header("Accept-Language", "fr-FR")
        .header("X-Discord-Locale", "fr")
        .header("X-Discord-Timezone", "Europe/Paris")
        .header("X-Debug-Options", "bugReporterEnabled")
        .header("User-Agent", "Discord-Android/192013;RNA")
        .header("Host", "discord.com")
        .header("Connection", "Keep-Alive")
        .header("Accept-Encoding", "gzip")
}

pub fn create_client () -> Client{
    Client::builder()
        .timeout(Duration::from_secs(60 * 60))
        .brotli(true)
        .gzip(true)
        .build()
        .unwrap()
}