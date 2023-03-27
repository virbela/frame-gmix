use super::codec;
use crate::{
    config::Config,
    message::{MessageRequest, MessageResponse, RequestMessage, ResponseMessage},
    mixer::session_manager::MixerSessionManager,
};
use futures::{future::poll_fn, ready, sink::Sink, stream::StreamExt};
use std::{
    collections::VecDeque,
    net::{IpAddr, SocketAddr, ToSocketAddrs},
    pin::Pin,
    str::FromStr,
    task::Poll,
    thread,
    time::Duration,
};
use tokio::spawn;
use tokio::{
    net::TcpStream,
    pin, select,
    time::{sleep, Instant},
};
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::trace;
use uuid::Uuid;

type Error = Box<dyn std::error::Error + Send + Sync>;

pub async fn handle_stream(mut stream: TcpStream, config: Config) -> Result<(), Error> {
    stream.set_nodelay(true)?;
    let (read, write) = stream.split();
    let mut read = FramedRead::new(read, codec::Server::default());
    let mut write = FramedWrite::new(write, codec::Server::default());

    let mut queue_write = QueuedWrite::new(&mut write);
    let server_id = config.clone().node;
    let timer = sleep(Duration::from_secs(10));
    pin!(timer);
    let register_response = ResponseMessage::OutgoingServer {
        node: Some(config.clone().node),
        message: MessageResponse::registerMixingServer {
            mode: config.clone().mode,
            region: "local".to_owned(),
        },
    };
    queue_write.push(register_response);
    loop {
        select! {
            opt = read.next() => {
                match opt {
                    Some(res) => {
                        let msg: RequestMessage = res?;
                        trace!("{msg:?}");
                        // handle request message.
                        match msg {
                        RequestMessage::IncomingServer { node, wsid, message } => {
                            println!("incoming message: {:?}", &message);
                            match message {
                            MessageRequest::createFrameAudioMixer { hello } => {
                                let port_range_start = (5000, 5100).0;
                                let port_range_end = (500, 5100).1;
                                let destination_port = 6000;
                                gstreamer::init()?;
                                let port_range = (port_range_start, port_range_end);
                                let mixer_manager = MixerSessionManager::new(port_range.clone());
                                let destination_ip1 = "127.0.0.1";
                                let session_id1 = "session1".to_string();
                                mixer_manager.create_session(session_id1.clone(), 2, destination_ip1, destination_port).unwrap();
                                let mixer_manager_clone = mixer_manager;
                                let session1_handle = thread::spawn(move || {
                                    let rt = tokio::runtime::Runtime::new().unwrap();
                                    rt.block_on(async move {
                                        mixer_manager_clone.start_session(&session_id1.clone()).await.unwrap();
                                    })
                                });
                                session1_handle.join().unwrap();
                                println!("createFrameAudioMixer {}", &hello);
                                let response = ResponseMessage::OutgoingServer {
                                    node: Some(server_id),
                                    message: MessageResponse::createdFrameAudioMixer {},
                                };
                                queue_write.push(response);
                            },
                            MessageRequest::destroyFrameAudioMixer {  } => {
                                println!("destroyedFrameAudioMixer");
                                let response = ResponseMessage::OutgoingServer {
                                    node: Some(server_id),
                                    message: MessageResponse::destroyFrameAudioMixer {},
                                };
                                queue_write.push(response);
                            },
                                }
                            },
                        }
                    },
                    None => return Ok(())
                }
            },
            res = queue_write.try_write() => res?,
            _ = timer.as_mut() => {
                // check for keep alive state.
                // send heart beat message.
                let response = ResponseMessage::OutgoingServer {
                    node: Some(server_id),
                    message: MessageResponse::serverLoad {
                        mode: config.clone().mode,
                        region: config.clone().mode,
                        load:  0.0
                    },
                };
                queue_write.push(response);

                // reset timer.
                timer.as_mut().reset(Instant::now() + Duration::from_secs(10));
            }
        }
    }
}

pub struct QueuedWrite<'a, S> {
    write: Pin<&'a mut S>,
    queue: VecDeque<ResponseMessage>,
    is_flush: bool,
}

impl<'a, S> QueuedWrite<'a, S>
where
    S: Sink<ResponseMessage, Error = std::io::Error> + Unpin,
{
    pub fn new(write: &'a mut S) -> Self {
        Self {
            write: Pin::new(write),
            queue: VecDeque::new(),
            is_flush: false,
        }
    }

    // push new message to queue.
    pub fn push(&mut self, msg: ResponseMessage) {
        self.queue.push_back(msg);
    }

    fn pop(&mut self) -> ResponseMessage {
        self.queue
            .pop_front()
            .expect("WriteQueue must not be operating io write on when empty")
    }

    // try to wire message in queue to io.
    pub async fn try_write(&mut self) -> Result<(), Error> {
        loop {
            match (self.is_flush, self.queue.is_empty()) {
                (false, true) => return poll_fn(|_| Poll::Pending).await,
                (false, false) => {
                    poll_fn(|cx| {
                        ready!(self.write.as_mut().poll_ready(cx))?;
                        let msg = self.pop();
                        Poll::Ready(self.write.as_mut().start_send(msg))
                    })
                    .await?;
                    self.is_flush = true;
                }
                (true, _) => {
                    poll_fn(|cx| self.write.as_mut().poll_flush(cx)).await?;
                    self.is_flush = false;
                }
            }
        }
    }
}
