#[macro_use]
extern crate strum_macros;

use std::time::Duration;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use serde_json::json;

use clap::Clap;


#[derive(Clap, Debug, Clone)]
pub struct Opts {
    #[clap(long)]
    pub mode: Mode,
    #[clap(long)]
    pub title: String,
    #[clap(long)]
    pub file_path: Option<String>,
    #[clap(long, default_value = "intern")]
    pub visibility: String,
    pub url: String,
    pub token: String,
    pub file_content: Option<String>,

}


#[derive(EnumString, Debug, Clone)]
pub enum Mode {
    Create,
    Update,
    Get,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct File {
    path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Snippet {
    title: String,
    pub id: usize,
    file_name: String,
    files: Vec<File>,
    web_url: String,
}


pub struct Api {
    pub config: Opts,
    client: Client,
}

impl GitLabApiClient for Api {}

impl Api {
    pub fn new() -> Api {
        let config: Opts = Opts::parse();
        let client = Api::create_client(&config).unwrap();
        Api{config, client}
    }

    pub async fn get_snippet(&self) -> Result<Snippet, Box<dyn std::error::Error>> {
        let resp = self.client.get(&self.config.url).send().await?;
        assert!(
            resp.status().is_success(),
            "HTTP GET Error {}: {}", resp.status(), resp.text().await.unwrap()
        );

        let data: Vec<Snippet> = resp.json().await?;
        let snippet_title = self.config.title.clone();
        match data.into_iter().filter(|s| s.title == snippet_title).next() {
            Some(snippet) => Ok(snippet),
            None => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Could not find snippet: {}", self.config.title),
            )))
        }
    }

    pub async fn create_snippet(&self) -> Result<Snippet, Box<dyn std::error::Error>> {
        let body = json!({
                "title": self.config.title,
                "description": "Autogenerated snippet",
                "visibility": &self.config.visibility,
                "files": [{
            "file_path": "init.txt",
            "content": &chrono::offset::Local::now().to_string()
            }]
        });

        let resp = self.client
            .post(&self.config.url)
            .json(&body)
            .send()
            .await?;

        assert!(
            resp.status().is_success(),
            "HTTP POST Error {}: {}", resp.status(), resp.text().await.unwrap()
        );
        let data = resp.json().await?;
        Ok(data)
    }

    pub async fn upload_file(&self, snippet_id: usize) -> Result<Snippet, Box<dyn std::error::Error>> {
        let body = json!({
            "files": [{
                "file_path": &self.config.file_path.as_ref().unwrap(),
                "content": &self.config.file_content.as_ref().unwrap(),
                "action": "create"
            }]
        });
        let url = format!("{}{}", &self.config.url, snippet_id);
        let resp = self.client
            .put(url)
            .json(&body)
            .send()
            .await?;

        assert!(
            resp.status().is_success(),
            "HTTP PUT Error {}: {}", resp.status(), resp.text().await.unwrap()
        );

        Ok(resp.json().await?)
    }
}

trait GitLabApiClient {
    fn create_client(config: &Opts) -> Result<Client, Box<dyn std::error::Error>, > {
        let mut headers = header::HeaderMap::new();
        let mut access_token = header::HeaderValue::from_str(
            config.token.as_str()
        )?;
        access_token.set_sensitive(true);
        headers.insert("PRIVATE-TOKEN", access_token);
        headers.insert(
            "Content-Type",
            header::HeaderValue::from_static("application/json"),
        );

        let client = Client::builder()
            .user_agent("Snipper")
            .default_headers(headers)
            .timeout(Duration::from_secs(10))
            .build()?;
        Ok(client)
    }
}


