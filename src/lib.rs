#![feature(async_closure)]
use tokio::net::{TcpStream, ToSocketAddrs};
use std::io;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::io::{AsyncWriteExt, ErrorKind};
use aqueue::{Actor, AResult};
use std::ops::Deref;
use std::sync::Arc;
use log::*;
use std::error::Error;
use aqueue::AError::{Other, StrErr};
use std::future::Future;

pub struct TcpClient {
    disconnect:bool,
    sender:OwnedWriteHalf
}


impl TcpClient {
    #[inline]
    pub async fn connect<T:ToSocketAddrs,F:Future<Output=Result<bool,Box<dyn Error>>>+Send+'static,A:Send+'static>(addr:T, f:impl FnOnce(A,Arc<Actor<TcpClient>>,OwnedReadHalf)->F+Send+'static,token:A) ->io::Result<Arc<Actor<TcpClient>>>{

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
    pub async fn disconnect(&mut self)->io::Result<()>{
        if !self.disconnect {
            self.sender.shutdown().await?;
            self.disconnect = true;
        }
        Ok(())
    }
    #[inline]
    pub async fn send(&mut self, buff:&[u8])->io::Result<usize>{
        if !self.disconnect {
            self.sender.write(buff).await
        }else{
            Err(io::Error::new(ErrorKind::Other,StrErr("Send Error,Disconnect".into())))
        }
    }


}

#[aqueue::aqueue_trait]
pub trait SocketClientTrait{
    async fn send<T:Deref<Target=[u8]>+Send+Sync+'static>(&self,buff:T)->AResult<usize>;
    async fn disconnect(&self)->AResult<()>;
}

#[aqueue::aqueue_trait]
impl SocketClientTrait for Actor<TcpClient>{
    #[inline]
    async fn send<T:Deref<Target=[u8]>+Send+Sync+'static>(&self, buff:T)->AResult<usize>{
        self.inner_call(async move |inner|{
            match inner.get_mut().send(&buff).await {
                Ok(size)=>Ok(size),
                Err(er)=>Err(Other(er.into()))
            }
        }).await
    }
    #[inline]
    async fn disconnect(&self) ->AResult<()> {
        self.inner_call(async move |inner| {
            match inner.get_mut().disconnect().await {
                Ok(_) => Ok(()),
                Err(er) => Err(Other(er.into()))
            }
        }).await

    }
}
