use clap::Parser;
use parse_display::{Display, FromStr};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tracing::{error, info};

mod format;

#[derive(Parser)]
struct App {
    kind: Kind,
}

#[derive(Display, FromStr, PartialEq, Debug)]
#[display(style = "snake_case")]
enum Kind {
    Client,
    Server,
}

#[tokio::main]
async fn main() -> Result<(), color_eyre::Report> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt::init();

    let app = App::parse();
    match app.kind {
        Kind::Client => run_client().await,
        Kind::Server => run_server().await,
    }
}

const ADDR: &str = "localhost:2777";

async fn run_server() -> Result<(), color_eyre::Report> {
    let ln = TcpListener::bind(ADDR).await?;
    info!("Listening on {ADDR}");

    loop {
        let (stream, addr) = ln.accept().await?;
        info!(%addr, "Accepted connection");

        tokio::spawn(async move {
            if let Err(e) = handle_client(stream).await {
                error!(%e, "Error while handling client conn")
            }
        });
    }
}

async fn handle_client(mut stream: TcpStream) -> Result<(), color_eyre::Report> {
    let num_records = 6;

    // write header
    let mut header = format::header::View::new(vec![0u8; format::header::SIZE.unwrap()]);
    header.flags_mut().write(0b0010);
    header.num_records_mut().write(num_records);
    stream.write_all(&header.into_storage()).await?;

    // re-use buffer when writing records
    let mut record_buf = vec![0u8; format::record::SIZE.unwrap()];
    for i in 0..num_records {
        let mut record = format::record::View::new(&mut record_buf[..]);
        record.value_mut().write(i as _);
        stream.write_all(record.into_storage()).await?;
    }

    Ok(())
}

async fn run_client() -> Result<(), color_eyre::Report> {
    info!("Connecting to {ADDR}");
    let mut stream = TcpStream::connect(ADDR).await?;

    let mut header_buf = vec![0u8; format::header::SIZE.unwrap()];
    stream.read_exact(&mut header_buf).await?;
    let header = format::header::View::new(&header_buf);
    let num_records = header.num_records().read();
    info!(
        "Got flags {:04b}, {num_records} records",
        header.flags().read()
    );

    // re-use buffer when reading records
    let mut record_buf = vec![0u8; format::record::SIZE.unwrap()];
    for i in 0..num_records {
        stream.read_exact(&mut record_buf).await?;
        let record = format::record::View::new(&record_buf);
        info!("Got record {}: {}", i, record.value().read());
    }

    Ok(())
}
