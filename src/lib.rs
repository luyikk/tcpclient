#![feature(async_closure)]
use anyhow::*;
use aqueue::Actor;
use log::*;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::{TcpStream, ToSocketAddrs};

pub struct TcpClient<T> {
    disconnect: bool,
    sender: WriteHalf<T>,
}

impl TcpClient<TcpStream> {
    #[inline]
    pub async fn connect<
        T: ToSocketAddrs,
        F: Future<Output = Result<bool>> + Send + 'static,
        A: Send + 'static,
    >(
        addr: T,
        f: impl FnOnce(A, Arc<Actor<TcpClient<TcpStream>>>, ReadHalf<TcpStream>) -> F + Send + 'static,
        token: A,
    ) -> Result<Arc<Actor<TcpClient<TcpStream>>>> {
        let stream = TcpStream::connect(addr).await?;
        let target = stream.peer_addr()?;
        Self::init(f, token, stream, target)
    }
}

impl<T> TcpClient<T>
where
    T: AsyncRead + AsyncWrite + Send + 'static,
{
    #[inline]
    pub async fn connect_stream_type<
        H: ToSocketAddrs,
        F: Future<Output = Result<bool>> + Send + 'static,
        S: Future<Output = Result<T>> + Send + 'static,
        A: Send + 'static,
    >(
        addr: H,
        f: impl FnOnce(A, Arc<Actor<TcpClient<T>>>, ReadHalf<T>) -> F + Send + 'static,
        i: impl FnOnce(TcpStream) -> S + Send + 'static,
        token: A,
    ) -> Result<Arc<Actor<TcpClient<T>>>> {
        let stream = TcpStream::connect(addr).await?;
        let target = stream.peer_addr()?;
        let stream = i(stream).await?;
        Self::init(f, token, stream, target)
    }

    #[inline]
    fn init<F: Future<Output = Result<bool>> + Send + 'static, A: Send + 'static>(
        f: impl FnOnce(A, Arc<Actor<TcpClient<T>>>, ReadHalf<T>) -> F + Send + 'static,
        token: A,
        stream: T,
        target: SocketAddr,
    ) -> Result<Arc<Actor<TcpClient<T>>>> {
        let (reader, sender) = tokio::io::split(stream);
        let client = Arc::new(Actor::new(TcpClient {
            disconnect: false,
            sender,
        }));
        let read_client = client.clone();
        tokio::spawn(async move {
            let disconnect_client = read_client.clone();
            let need_disconnect = match f(token, read_client, reader).await {
                Ok(disconnect) => disconnect,
                Err(err) => {
                    error!("reader error:{}", err);
                    true
                }
            };

            if need_disconnect {
                if let Err(er) = disconnect_client.disconnect().await {
                    error!("disconnect to{} err:{}", target, er);
                } else {
                    debug!("disconnect to {}", target)
                }
            } else {
                debug!("{} reader is close", target);
            }
        });
        Ok(client)
    }

    #[inline]
    pub async fn disconnect(&mut self) -> Result<()> {
        if !self.disconnect {
            self.sender.shutdown().await?;
            self.disconnect = true;
        }
        Ok(())
    }
    #[inline]
    pub async fn send<'a>(&'a mut self, buff: &'a [u8]) -> Result<usize> {
        if !self.disconnect {
            Ok(self.sender.write(buff).await?)
        } else {
            bail!("Send Error,Disconnect")
        }
    }

    #[inline]
    pub async fn send_all<'a>(&'a mut self, buff: &'a [u8]) -> Result<()> {
        if !self.disconnect {
            Ok(self.sender.write_all(buff).await?)
        } else {
            bail!("Send Error,Disconnect")
        }
    }

    #[inline]
    pub async fn flush(&mut self) -> Result<()> {
        if !self.disconnect {
            Ok(self.sender.flush().await?)
        } else {
            bail!("Send Error,Disconnect")
        }
    }
}

#[async_trait::async_trait]
pub trait SocketClientTrait {
    async fn send<'a>(&'a self, buff: &'a [u8]) -> Result<usize>;
    async fn send_all<'a>(&'a self, buff: &'a [u8]) -> Result<()>;
    async fn flush(&mut self) -> Result<()>;
    async fn disconnect(&self) -> Result<()>;
}

#[async_trait::async_trait]
impl<T> SocketClientTrait for Actor<TcpClient<T>>
where
    T: AsyncRead + AsyncWrite + Send + 'static,
{
    #[inline]
    async fn send<'a>(&'a self, buff: &'a [u8]) -> Result<usize> {
        unsafe {
            self.inner_call_ref(async move |inner| inner.get_mut().send(buff).await)
                .await
        }
    }
    #[inline]
    async fn send_all<'a>(&'a self, buff: &'a [u8]) -> Result<()> {
        unsafe {
            self.inner_call_ref(async move |inner| inner.get_mut().send_all(buff).await)
                .await
        }
    }

    #[inline]
    async fn flush(&mut self) -> Result<()> {
        self.inner_call(async move |inner| inner.get_mut().flush().await)
            .await
    }

    #[inline]
    async fn disconnect(&self) -> Result<()> {
        self.inner_call(async move |inner| inner.get_mut().disconnect().await)
            .await
    }
}
