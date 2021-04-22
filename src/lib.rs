#![feature(async_closure)]
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::io::AsyncWriteExt;
use aqueue::Actor;
use std::ops::Deref;
use std::sync::Arc;
use log::*;
use std::future::Future;
use anyhow::*;

pub struct TcpClient {
    disconnect:bool,
    sender:OwnedWriteHalf
}

impl TcpClient {
    #[inline]
    pub async fn connect<T:ToSocketAddrs,F:Future<Output=Result<bool>>+Send+'static,A:Send+'static>(addr:T, f:impl FnOnce(A,Arc<Actor<TcpClient>>,OwnedReadHalf)->F+Send+'static,token:A) ->Result<Arc<Actor<TcpClient>>>{

        let stream= TcpStream::connect(addr).await?;
        let target= stream.peer_addr()?;
        let(reader,sender)= stream.into_split();
        let client=Arc::new(Actor::new(TcpClient {
            disconnect:false,
            sender
        }));

        let read_client=client.clone();
        tokio::spawn(async move{
            let disconnect_client=read_client.clone();
            let need_disconnect=
                match f(token,read_client,reader).await{
                    Ok(disconnect)=>{
                        disconnect
                    },
                    Err(err)=>{
                        error!("reader error:{}",err);
                        true
                    }
                };

            if need_disconnect {
                if let Err(er)= disconnect_client.disconnect().await{
                    error!("disconnect to{} err:{}",target,er);
                }
                else{
                    debug!("disconnect to {}",target)
                }
            }
            else{
                debug!("{} reader is close",target);
            }
        });
        Ok(client)
    }
    #[inline]
    pub async fn disconnect(&mut self)->Result<()>{
        if !self.disconnect {
            self.sender.shutdown().await?;
            self.disconnect = true;
        }
        Ok(())
    }
    #[inline]
    pub async fn send(&mut self, buff:&[u8])->Result<usize>{
        if !self.disconnect {
            Ok(self.sender.write(buff).await?)
        }else{
            bail!("Send Error,Disconnect")
        }
    }


}

#[async_trait::async_trait]
pub trait SocketClientTrait{
    async fn send<T:Deref<Target=[u8]>+Send+Sync+'static>(&self,buff:T)->Result<usize>;
    async fn disconnect(&self)->Result<()>;
}

#[async_trait::async_trait]
impl SocketClientTrait for Actor<TcpClient>{
    #[inline]
    async fn send<T:Deref<Target=[u8]>+Send+Sync+'static>(&self, buff:T)->Result<usize>{
        self.inner_call(async move |inner|{
             inner.get_mut().send(&buff).await
        }).await
    }
    #[inline]
    async fn disconnect(&self) ->Result<()> {
        self.inner_call(async move |inner| {
             inner.get_mut().disconnect().await
        }).await

    }
}
