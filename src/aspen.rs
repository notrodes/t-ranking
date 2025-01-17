use regex::Regex;
use reqwest::{self, Client};
use thiserror::Error;

use reqwest::header;
use std::env;

#[derive(Clone, Debug)]
pub struct AspenInfo {
    client: Client,
    pub session_id: String,
    pub apache_token: String,
}

impl AspenInfo {
    pub async fn new() -> Result<AspenInfo, ProjError> {
        let client = reqwest::Client::new();
        let [session_id, apache_token] = AspenInfo::get_session(&client).await?;
        Ok(AspenInfo {
            client,

            session_id,
            apache_token,
        })
    }

    // Request a session id from aspen for later use [session_id, apache_token]
    async fn get_session(client: &Client) -> Result<[String; 2], ProjError> {
        let res = client
            .get("https://aspen.cpsd.us/aspen/logon.do")
            .send()
            .await?
            .text()
            .await?;
        let mut ret = [String::default(), String::default()];
        for (i, pattern) in [
            "sessionId='(.+)';", // Regex for finding session id in res (regex from https://github.com/Aspine/aspine/blob/master/src/scrape.ts:762)
            "name=\"org.apache.struts.taglib.html.TOKEN\" value=\"(.+)\"", // Regex for finding apache token in res (regex from https://github.com/Aspine/aspine/blob/master/src/scrape.ts:766)
        ]
        .iter()
        .enumerate()
        {
            ret[i] = Regex::new(pattern)
                .unwrap()
                .captures(&res)
                .ok_or(ProjError::from(AspenError::NoSession))?
                .get(1)
                .ok_or(ProjError::from(AspenError::NoSession))?
                .as_str()
                .to_owned()
        }
        Ok(ret)
    }
}

#[derive(Error, Debug)]
pub enum ProjError {
    #[error("AspenError")]
    Aspen(#[from] AspenError),
    #[error("Network error")]
    NetworkError(#[from] reqwest::Error),
}

#[derive(Error, Debug)]
pub enum AspenError {
    #[error("NoSession Error, Invalid Response Returned")]
    NoSession,
    #[error("InvalidLogin Error, Please Try Again")]
    InvalidLogin,
}

pub async fn get_aspen() -> Result<String, ProjError> {
    // In future, get user credentials from frontend
    let username = env::var("ASPEN_USERNAME").unwrap();
    let password = env::var("ASPEN_PASSWORD").unwrap();
    let client = reqwest::Client::builder().build()?;
    // Getting session_id and apache_token from Aspen
    let info = AspenInfo::new().await?;
    // Login to aspen
    let login_res = client
        .post("https://aspen.cpsd.us/aspen/logon.do")
        .header(
            header::COOKIE,
            format!("JSESSIONID={}.aspen-app2", info.session_id),
        )
        .header(header::USER_AGENT, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/96.0.4664.45 Safari/537.36")
        .query(&[("org.apache.struts.taglib.html.TOKEN", info.apache_token)])
        .query(&[("userEvent", "930")])
        .query(&[("deploymentId", "x2sis")])
        .query(&[("username", username)])
        .query(&[("password", password)])
        .send()
        .await?
        .text()
        .await?;
    // Check if login was successful
    if login_res.contains("Invalid login.") {
        return Err(ProjError::Aspen(AspenError::InvalidLogin));
    }
    // TODO: see aspine's get_academics() and get_class_details() in src/scrape.ts
    // Sample request, getting list of classes
    let res = client
        .get("https://aspen.cpsd.us/aspen/portalClassList.do?navkey=academics.classes.list")
        .header("Cookie", format!("JSESSIONID={}", info.session_id))
        .send()
        .await?
        .text()
        .await?;
    Ok(res)
}
