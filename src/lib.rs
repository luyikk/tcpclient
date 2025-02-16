pub mod error;
use aqueue::Actor;
use error::Result;
use log::*;
use std::future::Future;
use std::net::SocketAddr;
use std::ops::Deref;
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
        F: Future<Output = anyhow::Result<bool>> + Send + 'static,
        A: Send + 'static,
    >(
        addr: T,
        input: impl FnOnce(A, Arc<Actor<TcpClient<TcpStream>>>, ReadHalf<TcpStream>) -> F
            + Send
            + 'static,
        token: A,
    ) -> Result<Arc<Actor<TcpClient<TcpStream>>>> {
        let stream = TcpStream::connect(addr).await?;
        let target = stream.peer_addr()?;
        Self::init(input, token, stream, target)
    }
}

impl<T> TcpClient<T>
where
    T: AsyncRead + AsyncWrite + Send + 'static,
{
    #[inline]
    pub async fn connect_stream_type<
        H: ToSocketAddrs,
        F: Future<Output = anyhow::Result<bool>> + Send + 'static,
        S: Future<Output = anyhow::Result<T>> + Send + 'static,
        A: Send + 'static,
    >(
        addr: H,
        stream_init: impl FnOnce(TcpStream) -> S + Send + 'static,
        input: impl FnOnce(A, Arc<Actor<TcpClient<T>>>, ReadHalf<T>) -> F + Send + 'static,
        token: A,
    ) -> Result<Arc<Actor<TcpClient<T>>>> {
        let stream = TcpStream::connect(addr).await?;
        let target = stream.peer_addr()?;
        let stream = stream_init(stream).await?;
        Self::init(input, token, stream, target)
    }

    #[inline]
    fn init<F: Future<Output = anyhow::Result<bool>> + Send + 'static, A: Send + 'static>(
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
            let need_disconnect = f(token, read_client, reader).await.unwrap_or_else(|err| {
                error!("reader error:{}", err);
                true
            });

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
    pub async fn send(&mut self, buff: &[u8]) -> Result<usize> {
        if !self.disconnect {
            Ok(self.sender.write(buff).await?)
        } else {
            Err(error::Error::SendError("Disconnect".to_string()))
        }
    }

    #[inline]
    async fn send_all(&mut self, buff: &[u8]) -> Result<()> {
        if !self.disconnect {
            self.sender.write_all(buff).await?;
            Ok(self.sender.flush().await?)
        } else {
            Err(error::Error::SendError("Disconnect".to_string()))
        }
    }

    #[inline]
    pub async fn flush(&mut self) -> Result<()> {
        if !self.disconnect {
            Ok(self.sender.flush().await?)
        } else {
            Err(error::Error::SendError("Disconnect".to_string()))
        }
    }
}

pub trait SocketClientTrait {
    fn send<B: Deref<Target = [u8]> + Send + Sync + 'static>(
        &self,
        buff: B,
    ) -> impl Future<Output = Result<usize>>;
    fn send_all<B: Deref<Target = [u8]> + Send + Sync + 'static>(
        &self,
        buff: B,
    ) -> impl Future<Output = Result<()>>;
    fn send_ref(&self, buff: &[u8]) -> impl Future<Output = Result<usize>>;
    fn send_all_ref(&self, buff: &[u8]) -> impl Future<Output = Result<()>>;
    fn flush(&self) -> impl Future<Output = Result<()>>;
    fn disconnect(&self) -> impl Future<Output = Result<()>>;
}

impl<T> SocketClientTrait for Actor<TcpClient<T>>
where
    T: AsyncRead + AsyncWrite + Send + 'static,
{
    #[inline]
    async fn send<B: Deref<Target = [u8]> + Send + Sync + 'static>(
        &self,
        buff: B,
    ) -> Result<usize> {
        self.inner_call(|inner| async move { inner.get_mut().send(&buff).await })
            .await
    }
    #[inline]
    async fn send_all<B: Deref<Target = [u8]> + Send + Sync + 'static>(
        &self,
        buff: B,
    ) -> Result<()> {
        self.inner_call(|inner| async move { inner.get_mut().send_all(&buff).await })
            .await
    }
    #[inline]
    async fn send_ref(&self, buff: &[u8]) -> Result<usize> {
        if buff.is_empty() {
            return Err(error::Error::SendError("send buff is none".to_string()));
        }
        self.inner_call(|inner| async move { inner.get_mut().send(buff).await })
            .await
    }

    #[inline]
    async fn send_all_ref(&self, buff: &[u8]) -> Result<()> {
        if buff.is_empty() {
            return Err(error::Error::SendError("send buff is none".to_string()));
        }
        self.inner_call(|inner| async move { inner.get_mut().send_all(buff).await })
            .await
    }

    #[inline]
    async fn flush(&self) -> Result<()> {
        self.inner_call(|inner| async move { inner.get_mut().flush().await })
            .await
    }

    #[inline]
    async fn disconnect(&self) -> Result<()> {
        self.inner_call(|inner| async move { inner.get_mut().disconnect().await })
            .await
    }
}
