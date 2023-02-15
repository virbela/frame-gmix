use crate::message::{RequestMessage, ResponseMessage};

use super::codec;
use futures::{future::poll_fn, ready, sink::Sink, stream::StreamExt};
use std::{
    collections::VecDeque,
    net::{IpAddr, SocketAddr, ToSocketAddrs},
    pin::Pin,
    str::FromStr,
    task::Poll,
    time::Duration,
};
use tokio::{
    net::TcpStream,
    pin, select,
    time::{sleep, Instant},
};
use tokio_util::codec::{FramedRead, FramedWrite};
use uuid::Uuid;

type Error = Box<dyn std::error::Error + Send + Sync>;

pub async fn handle_stream(mut stream: TcpStream) -> Result<(), Error> {
    stream.set_nodelay(true)?;

    let (read, write) = stream.split();
    let mut read = FramedRead::new(read, codec::Server::default());
    let mut write = FramedWrite::new(write, codec::Server::default());

    let mut queue_write = QueuedWrite::new(&mut write);
    let server_id = Uuid::parse_str("fe3219b4-ad05-11ed-afa1-0242ac120002").unwrap();
    let timer = sleep(Duration::from_secs(10));
    pin!(timer);

    loop {
        select! {
            opt = read.next() => {
                match opt {
                    Some(res) => {
                        let msg: RequestMessage = res?;

                        // handle request message.
                    },
                    None => return Ok(())
                }
            }
            res = queue_write.try_write() => res?,
            _ = timer.as_mut() => {
                // check for keep alive state.

                // send heart beat message.
                let response = ResponseMessage::OutgoingServer {
                    node: Some(server_id),
                    message: crate::message::MessageResponse::Ping,
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
