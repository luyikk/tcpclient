# Asynchronous tcpclient based on aqueue actor.

## Examples echo
add dependencies
```toml
[dependencies]
tokio = { version = "0.2", features = ["full"] }
aqueue="0.1.18"
bytes="0.6"
log="0.4"
env_logger = "0.8.2"
```
main.rs
```rust
#![feature(async_closure)]
use tcpclient::{TcpClient,SocketClientTrait};
use bytes::{BytesMut, BufMut};
use tokio::io::AsyncReadExt;
use std::error::Error;
use log::LevelFilter;

#[tokio::main]
async fn main()->Result<(),Box<dyn Error>> {
    // set logger out
    env_logger::Builder::new().filter_level(LevelFilter::Debug).init();
    
    // connect echo server
    let client=
        TcpClient::connect("127.0.0.1:1002", async move|client, mut reader| {
            // read i32 from target server
            while let Ok(len) = reader.read_i32_le().await {
                // send i32 to target server
                let mut buff = BytesMut::new();
                buff.put_i32_le(len);
                client.send(buff).await?;
            }
            // return true need disconnect,false not disconnect
            // if true and the current state is disconnected, it will be ignored.
            Ok(true)

        }).await?;

    // connect ok send i32 to target server
    let mut buff =BytesMut::new();
    buff.put_i32_le(4);
    client.send(buff).await?;
    
    // test disconnect readline 
    let mut str="".into();
    std::io::stdin().read_line(&mut str)?;
    
    // disconnect target server
    client.disconnect().await?;
    // wait env logger out show
    std::io::stdin().read_line(&mut str)?;
    Ok(())
}
```