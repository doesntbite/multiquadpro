use crate::config::Config;
use std::pin::Pin;
use std::task::{Context, Poll};
use bytes::{BufMut, BytesMut};
use futures_util::Stream;
use pin_project_lite::pin_project;
use pretty_bytes::converter::convert;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::time::{timeout, Duration};
use worker::*;

// Configuration constants
const INITIAL_BUFFER_SIZE: usize = 8 * 1024; // 8KB
const MAX_WEBSOCKET_SIZE: usize = 128 * 1024; // 128kb (increased from 64kb)
const MAX_BUFFER_SIZE: usize = 4 * 1024 * 1024; // 4MB (increased from 512kb)
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const IO_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_RETRIES: usize = 3;

pin_project! {
    pub struct ProxyStream<'a> {
        pub config: Config,
        pub ws: &'a WebSocket,
        pub buffer: BytesMut,
        #[pin]
        pub events: EventStream<'a>,
        pub backpressure_flag: bool,
    }
}

impl<'a> ProxyStream<'a> {
    pub fn new(config: Config, ws: &'a WebSocket, events: EventStream<'a>) -> Self {
        let buffer = BytesMut::with_capacity(INITIAL_BUFFER_SIZE);

        Self {
            config,
            ws,
            buffer,
            events,
            backpressure_flag: false,
        }
    }
    
    pub async fn fill_buffer_until(&mut self, n: usize) -> std::io::Result<()> {
        use futures_util::StreamExt;

        let mut retries = 0;
        while self.buffer.len() < n && retries < MAX_RETRIES {
            match self.events.next().await {
                Some(Ok(WebsocketEvent::Message(msg))) => {
                    if let Some(data) = msg.bytes() {
                        if data.len() > MAX_WEBSOCKET_SIZE {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "websocket message too large",
                            ));
                        }
                        
                        if self.buffer.len() + data.len() > MAX_BUFFER_SIZE {
                            self.backpressure_flag = true;
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::WouldBlock,
                                "buffer full, applying backpressure",
                            ));
                        }
                        
                        self.buffer.put_slice(&data);
                        retries = 0; // Reset retries on successful read
                    }
                }
                Some(Ok(WebsocketEvent::Close(_))) => {
                    break;
                }
                Some(Err(e)) => {
                    retries += 1;
                    tokio::time::sleep(Duration::from_millis(100 * retries as u64)).await;
                    continue;
                }
                None => {
                    retries += 1;
                    tokio::time::sleep(Duration::from_millis(100 * retries as u64)).await;
                    continue;
                }
            }
        }
        
        if self.buffer.len() < n {
            Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "failed to fill buffer",
            ))
        } else {
            Ok(())
        }
    }

    pub fn peek_buffer(&self, n: usize) -> &[u8] {
        let len = self.buffer.len().min(n);
        &self.buffer[..len]
    }

    pub async fn process(&mut self) -> Result<()> {
        let peek_buffer_len = 62;
        match self.fill_buffer_until(peek_buffer_len).await {
            Ok(_) => {
                let peeked_buffer = self.peek_buffer(peek_buffer_len);

                if peeked_buffer.len() < (peek_buffer_len/2) {
                    return Err(Error::RustError("not enough buffer".to_string()));
                }

                if self.is_vless(peeked_buffer) {
                    console_log!("vless detected!");
                    self.process_vless().await
                } else if self.is_shadowsocks(peeked_buffer) {
                    console_log!("shadowsocks detected!");
                    self.process_shadowsocks().await
                } else if self.is_trojan(peeked_buffer) {
                    console_log!("trojan detected!");
                    self.process_trojan().await
                } else if self.is_vmess(peeked_buffer) {
                    console_log!("vmess detected!");
                    self.process_vmess().await
                } else {
                    Err(Error::RustError("protocol not implemented".to_string()))
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    // Send backpressure signal
                    self.ws.send(Message::Ping(vec![])).map_err(|e| {
                        Error::RustError(format!("backpressure signal failed: {}", e))
                    })?;
                    Err(Error::RustError("backpressure applied".to_string()))
                } else {
                    Err(Error::RustError(format!("buffer fill error: {}", e)))
                }
            }
        }
    }

    pub fn is_vless(&self, buffer: &[u8]) -> bool {
        buffer.len() > 0 && buffer[0] == 0
    }

    fn is_shadowsocks(&self, buffer: &[u8]) -> bool {
        if buffer.is_empty() {
            return false;
        }
        
        match buffer[0] {
            1 => { // IPv4
                if buffer.len() < 7 {
                    return false;
                }
                let remote_port = u16::from_be_bytes([buffer[5], buffer[6]]);
                remote_port != 0
            }
            3 => { // Domain name
                if buffer.len() < 2 {
                    return false;
                }
                let domain_len = buffer[1] as usize;
                if buffer.len() < 2 + domain_len + 2 {
                    return false;
                }
                let remote_port = u16::from_be_bytes([
                    buffer[2 + domain_len],
                    buffer[2 + domain_len + 1],
                ]);
                remote_port != 0
            }
            4 => { // IPv6
                if buffer.len() < 19 {
                    return false;
                }
                let remote_port = u16::from_be_bytes([buffer[17], buffer[18]]);
                remote_port != 0
            }
            _ => false,
        }
    }

    fn is_trojan(&self, buffer: &[u8]) -> bool {
        buffer.len() > 57 && buffer[56] == 13 && buffer[57] == 10
    }

    fn is_vmess(&self, buffer: &[u8]) -> bool {
        !buffer.is_empty()
    }

    pub async fn handle_tcp_outbound(&mut self, addr: String, port: u16) -> Result<()> {
        let mut remote_socket = timeout(CONNECT_TIMEOUT, 
            Socket::builder().connect(&addr, port)
        ).await.map_err(|_| {
            Error::RustError(format!("connection timeout to {}:{}", addr, port))
        })??;

        timeout(IO_TIMEOUT, remote_socket.opened()).await.map_err(|_| {
            Error::RustError(format!("socket open timeout to {}:{}", addr, port))
        })??;

        let (bytes_up, bytes_down) = timeout(IO_TIMEOUT, 
            tokio::io::copy_bidirectional(self, &mut remote_socket)
        ).await.map_err(|_| {
            Error::RustError(format!("i/o timeout with {}:{}", addr, port))
        })??;

        console_log!(
            "connection to {}:{} completed, uploaded: {}, downloaded: {}",
            addr,
            port,
            convert(bytes_up as f64),
            convert(bytes_down as f64)
        );

        Ok(())
    }

    pub async fn handle_udp_outbound(&mut self) -> Result<()> {
        let mut buff = vec![0u8; 65535];

        let n = timeout(IO_TIMEOUT, self.read(&mut buff)).await??;
        let data = &buff[..n];
        
        if crate::dns::doh(data).await.is_ok() {
            timeout(IO_TIMEOUT, self.write(&data)).await??;
        };
        
        Ok(())
    }

    fn cleanup_buffer(&mut self) {
        if self.buffer.capacity() > MAX_BUFFER_SIZE * 2 {
            self.buffer = BytesMut::with_capacity(MAX_BUFFER_SIZE);
        } else if self.buffer.capacity() > MAX_BUFFER_SIZE && self.buffer.len() < MAX_BUFFER_SIZE / 4 {
            self.buffer.reserve(MAX_BUFFER_SIZE);
        }
    }
}

impl<'a> AsyncRead for ProxyStream<'a> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<tokio::io::Result<()>> {
        let mut this = self.project();
        this.cleanup_buffer();

        loop {
            let size = std::cmp::min(this.buffer.len(), buf.remaining());
            if size > 0 {
                buf.put_slice(&this.buffer.split_to(size));
                *this.backpressure_flag = false;
                return Poll::Ready(Ok(()));
            }

            match this.events.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(WebsocketEvent::Message(msg)))) => {
                    if let Some(data) = msg.bytes() {
                        if data.len() > MAX_WEBSOCKET_SIZE {
                            return Poll::Ready(Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "websocket message too large",
                            )));
                        }
                        
                        if this.buffer.len() + data.len() > MAX_BUFFER_SIZE {
                            *this.backpressure_flag = true;
                            if let Err(e) = this.ws.send(Message::Ping(vec![])) {
                                return Poll::Ready(Err(std::io::Error::new(
                                    std::io::ErrorKind::ConnectionAborted,
                                    e.to_string(),
                                )));
                            }
                            return Poll::Pending;
                        }
                        
                        this.buffer.put_slice(&data);
                    }
                }
                Poll::Pending => {
                    if *this.backpressure_flag {
                        // Send another ping if we're still in backpressure mode
                        if let Err(e) = this.ws.send(Message::Ping(vec![])) {
                            return Poll::Ready(Err(std::io::Error::new(
                                std::io::ErrorKind::ConnectionAborted,
                                e.to_string(),
                            )));
                        }
                    }
                    return Poll::Pending;
                }
                _ => return Poll::Ready(Ok(())),
            }
        }
    }
}

impl<'a> AsyncWrite for ProxyStream<'a> {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<tokio::io::Result<usize>> {
        let this = self.project();
        match this.ws.send_with_bytes(buf) {
            Ok(_) => Poll::Ready(Ok(buf.len())),
            Err(e) => Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<tokio::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<tokio::io::Result<()>> {
        match self.ws.close(Some(1000), Some("normal shutdown".to_string())) {
            Ok(_) => Poll::Ready(Ok(())),
            Err(e) => Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))),
        }
    }
}

impl<'a> Drop for ProxyStream<'a> {
    fn drop(&mut self) {
        let _ = self.ws.close(1000, "proxy stream dropped");
    }
}
