# Asynchronous tcpclient based on aqueue actor.

## DEMO URL:
## [demo url][https://github.com/luyikk/tcp_server/tree/master/examples]

## examples echo:
```rust
#![feature(async_closure)]
use tcpclient::{TcpClient,SocketClientTrait};
use tokio::io::AsyncReadExt;
use std::error::Error;
use log::LevelFilter;

#[tokio::main]
async fn main()->Result<(),Box<dyn Error>> {
    // set logger out
    env_logger::Builder::new().filter_level(LevelFilter::Debug).init();

    // connect echo server
    let client =
        TcpClient::connect("127.0.0.1:5555", async move |_, client, mut reader| {
            // read buff from target server
            let mut buff=[0;7];
            while let Ok(len) = reader.read_exact(&mut buff).await {
                // send buff to target server
                println!("{}",std::str::from_utf8(&buff[..len])?);
                client.send(&buff[..len]).await?;
            }
            // return true need disconnect,false not disconnect
            // if true and the current state is disconnected, it will be ignored.
            Ok(true)
        }, ()).await?;

    // connect ok send buff to target server
    client.send(b"1234567").await?;

    // test disconnect readline 
    let mut str = "".into();
    std::io::stdin().read_line(&mut str)?;

    // disconnect target server
    client.disconnect().await?;
    // wait env logger out show
    std::io::stdin().read_line(&mut str)?;
    Ok(())
}
```


[https://github.com/luyikk/tcp_server/tree/master/examples]: https://github.com/luyikk/tcp_server/tree/master/examples