use bytes::Bytes;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::sync::Mutex;
use tracing::{error, info, trace};
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;

const BUFFER_SIZE: usize = 4096;

pub fn spawn_dc_socket_bridge<S>(dc: Arc<RTCDataChannel>, socket: S)
where
    S: AsyncRead + AsyncWrite + Send + Sync + 'static,
{
    let (socket_read, socket_write) = tokio::io::split(socket);
    let socket_write = Arc::new(Mutex::new(socket_write));

    let dc_for_open = Arc::clone(&dc);
    dc.on_open(Box::new(move || {
        let dc = Arc::clone(&dc_for_open);
        Box::pin(async move {
            info!("{} opened", dc.label());
            tokio::spawn(socket_to_dc(dc, socket_read));
        })
    }));

    let dc_for_msg = Arc::clone(&dc);
    let socket_for_msg = Arc::clone(&socket_write);
    dc.on_message(Box::new(move |msg: DataChannelMessage| {
        let dc = Arc::clone(&dc_for_msg);
        let socket = Arc::clone(&socket_for_msg);
        Box::pin(async move {
            if let Err(e) = dc_to_socket(msg.data.to_vec(), &socket).await {
                error!("{} write error: {}", dc.label(), e);
                let _ = dc.close().await;
            }
        })
    }));

    let dc_label = dc.label().to_string();
    dc.on_error(Box::new(move |err: webrtc::Error| {
        let label = dc_label.clone();
        Box::pin(async move {
            error!("{} error: {}", label, err);
        })
    }));

    let dc_for_close = Arc::clone(&dc);
    let socket_for_close = Arc::clone(&socket_write);
    dc.on_close(Box::new(move || {
        let dc = Arc::clone(&dc_for_close);
        let socket = Arc::clone(&socket_for_close);
        Box::pin(async move {
            info!("{} closed", dc.label());
            let mut guard = socket.lock().await;
            let _ = guard.shutdown().await;
        })
    }));
}

async fn socket_to_dc<R>(dc: Arc<RTCDataChannel>, mut reader: ReadHalf<R>)
where
    R: AsyncRead + Send + Sync,
{
    let mut buffer = vec![0u8; BUFFER_SIZE];

    loop {
        match reader.read(&mut buffer).await {
            Ok(0) => {
                info!("{} socket EOF. closed", dc.label());
                break;
            }
            Ok(n) => {
                let bytes = Bytes::copy_from_slice(&buffer[..n]);
                if let Err(e) = dc.send(&bytes).await {
                    error!("{} send error: {}", dc.label(), e);
                    break;
                }
                trace!("{} {} bytes -> DC", dc.label(), n);
            }
            Err(e) => {
                error!("{} socket read error: {}", dc.label(), e);
                break;
            }
        }
    }

    let _ = dc.close().await;
}

async fn dc_to_socket<W>(data: Vec<u8>, writer: &Arc<Mutex<WriteHalf<W>>>) -> std::io::Result<()>
where
    W: AsyncWrite + Send + Sync,
{
    let mut guard = writer.lock().await;
    guard.write_all(&data).await
}
