use anyhow::Result;
use textbelt::TextbeltClient;
use log::{info,debug,error};

#[derive(Clone)]
pub struct TextbeltSink {
    phone: String,
    api_key: String,
}



impl TextbeltSink {
    pub fn new(api_key: String, phone: String) -> Self {
        Self { phone, api_key }
    }
    pub async fn send(&self, msg: &str) -> Result<()> {
        info!("Sending notification from SMS...");
        let tc = TextbeltClient::new(&self.api_key,  Some("dende-rs"));
        let response = tc.text(&self.phone, msg).await?;
        info!("Response: {:?}", response);
        Ok(())
    }
}
