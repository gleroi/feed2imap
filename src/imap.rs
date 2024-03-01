use anyhow::Error;
use async_imap::types::Fetch;
use async_native_tls::TlsStream;
use futures::StreamExt;
use mail_parser;
use std::{collections::BTreeSet, sync::Arc};
use tokio::{net::TcpStream, sync::Mutex};

use crate::sync;

pub struct Client {
    imap: async_imap::Session<TlsStream<TcpStream>>,
}

pub async fn client(username: &str, password: &str) -> Result<Client, Error> {
    let imap_server = "mail.infomaniak.com";
    let imap_addr = (imap_server, 993);
    let tcp_stream = TcpStream::connect(imap_addr).await?;
    let tls = async_native_tls::TlsConnector::new();
    let tls_stream = tls.connect(imap_server, tcp_stream).await?;
    let client = async_imap::Client::new(tls_stream);
    let session = client.login(username, password).await.map_err(|e| e.0)?;
    Ok(Client { imap: session })
}

impl Client {
    pub async fn append(&mut self, mail: &Vec<u8>, folder: &str) -> Result<(), Error> {
        let _result = self.imap.select(folder).await?;
        let _result = self.imap.append(folder, mail).await?;
        Ok(())
    }

    pub async fn logout(&mut self) -> Result<(), Error> {
        let _result = self.imap.logout().await?;
        Ok(())
    }

    pub async fn list_message_ids(&mut self, folder: &str) -> Result<BTreeSet<String>, Error> {
        let mailbox = self.imap.examine(folder).await?;
        log::debug!("there is {} in {} mailbox", mailbox.exists, folder);

        let stream = self
            .imap
            .fetch("1:*", "(FLAGS BODY.PEEK[HEADER.FIELDS (MESSAGE-ID)])")
            .await?;
        let fetches: Result<Vec<Fetch>, _> = stream.collect::<Vec<_>>().await.into_iter().collect();
        let parser = mail_parser::MessageParser::default();
        let ids = fetches?
            .iter()
            .filter_map(|fetch| fetch.header())
            .filter_map(|header| parser.parse_headers(header))
            .filter_map(|msg| {
                if let Some(header) = msg.header("MESSAGE-ID") {
                    if let Some(id) = header.clone().into_text() {
                        Some(id.into_owned())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        Ok(ids)
    }
}

pub struct InnerOutput {
    client: Mutex<Client>,
    ids: BTreeSet<String>,
    folder: String,
}

pub type Output = Arc<InnerOutput>;

pub async fn new_output(mut client: Client, folder: &str) -> Result<Output, Error> {
    let ids = client.list_message_ids(&folder).await?;
    Ok(Arc::new(InnerOutput {
        client: Mutex::new(client),
        ids,
        folder: folder.to_string(),
    }))
}

impl sync::Output for Output {
    fn contains(&self, id: &str) -> bool {
        return self.ids.contains(id);
    }

    async fn append(&self, mail: &Vec<u8>) -> Result<(), Error> {
        let mut imap_client = self.client.lock().await;
        Ok(imap_client.append(mail, &self.folder).await?)
    }
}
