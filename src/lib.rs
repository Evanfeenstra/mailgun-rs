use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;

const MAILGUN_API: &str = "https://api.mailgun.net/v3";
// eu: https://api.eu.mailgun.net/v3
const MESSAGES_ENDPOINT: &str = "messages";

#[derive(Default)]
pub struct Mailgun {
    pub domain: String,
    pub api_key: String,
    pub zone: Option<String>,
}

pub type SendResult<T> = Result<T, anyhow::Error>;

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct SendResponse {
    pub message: String,
    pub id: String,
}

impl Mailgun {
    pub fn new(domain: &str, api_key: &str) -> Self {
        Self {
            domain: domain.to_string(),
            api_key: api_key.to_string(),
            zone: None,
        }
    }
    pub fn set_zone(&mut self, zone: &str) {
        self.zone = Some(zone.to_string());
    }
    pub async fn send(self, sender: &EmailAddress, msg: Message) -> SendResult<SendResponse> {
        let client = reqwest::Client::new();
        let mut params = msg.params();
        params.insert("from".to_string(), sender.to_string());
        let root = self.zone.unwrap_or(MAILGUN_API.to_string());
        let url = format!("{}/{}/{}", &root, self.domain, MESSAGES_ENDPOINT);

        let res = client
            .post(url)
            .basic_auth("api", Some(self.api_key))
            .form(&params)
            .send()
            .await?;
        if res.status().is_success() {
            let parsed: SendResponse = res.json().await?;
            Ok(parsed)
        } else {
            let parsed = res.text().await?;
            Err(anyhow::anyhow!("{:?}", parsed))
        }
    }
}

#[derive(Default, Debug)]
pub struct Message {
    pub to: Vec<EmailAddress>,
    pub cc: Vec<EmailAddress>,
    pub bcc: Vec<EmailAddress>,
    pub subject: String,
    pub text: String,
    pub html: String,
    pub template: String,
    pub template_vars: HashMap<String, String>,
    pub recipient_vars: HashMap<String, HashMap<String, String>>,
}

impl Message {
    fn params(self) -> HashMap<String, String> {
        let mut params = HashMap::new();

        Message::add_recipients("to", self.to, &mut params);
        Message::add_recipients("cc", self.cc, &mut params);
        Message::add_recipients("bcc", self.bcc, &mut params);

        params.insert(String::from("subject"), self.subject);

        params.insert(String::from("text"), self.text);
        params.insert(String::from("html"), self.html);

        // add template
        if !self.template.is_empty() {
            params.insert(String::from("template"), self.template);
            if !self.template_vars.is_empty() {
                params.insert(
                    String::from("h:X-Mailgun-Variables"),
                    serde_json::to_string(&self.template_vars).unwrap(),
                );
            }
            if !self.recipient_vars.is_empty() {
                params.insert(
                    String::from("h:X-Mailgun-Recipient-Variables"),
                    serde_json::to_string(&self.recipient_vars).unwrap(),
                );
            }
        }

        params
    }

    fn add_recipients(
        field: &str,
        addresses: Vec<EmailAddress>,
        params: &mut HashMap<String, String>,
    ) {
        if !addresses.is_empty() {
            let joined = addresses
                .iter()
                .map(EmailAddress::to_string)
                .collect::<Vec<String>>()
                .join(",");
            params.insert(field.to_owned(), joined);
        }
    }
}

#[derive(Debug)]
pub struct EmailAddress {
    name: Option<String>,
    address: String,
}

impl EmailAddress {
    pub fn address(address: &str) -> Self {
        EmailAddress {
            name: None,
            address: address.to_string(),
        }
    }

    pub fn name_address(name: &str, address: &str) -> Self {
        EmailAddress {
            name: Some(name.to_string()),
            address: address.to_string(),
        }
    }
}

impl fmt::Display for EmailAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.name {
            Some(ref name) => write!(f, "{} <{}>", name, self.address),
            None => write!(f, "{}", self.address),
        }
    }
}
