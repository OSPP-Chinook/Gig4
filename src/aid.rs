use std::{sync::mpsc, thread};

// actor identifier
#[derive(Clone)]
pub struct AID<T> {
    pub tid: thread::ThreadId,
    channel: mpsc::Sender<T>,
}

impl<T> AID<T> {
    // creates a channel and spawns a thread running f
    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce(AID<T>, mpsc::Receiver<T>),
        F: Send + 'static,
        T: Send + 'static,
    {
        let (sender, reciever) = mpsc::channel();
        let handle = thread::spawn({
            let sender = sender.clone();
            || {
                let aid = AID {
                    tid: thread::current().id(),
                    channel: sender,
                };
                f(aid, reciever)
            }
        });
        AID {
            tid: handle.thread().id(),
            channel: sender,
        }
    }

    pub fn send(&self, t: T) -> Result<(), mpsc::SendError<T>> {
        self.channel.send(t)
    }
}
