use anyhow::Error;
use async_native_tls::TlsStream;
use tokio::net::TcpStream;

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
        let _result = self.imap.append(folder, mail).await?;
        Ok(())
    }

    pub async fn logout(&mut self) -> Result<(), Error> {
        let _result = self.imap.logout().await?;
        Ok(())
    }
}
