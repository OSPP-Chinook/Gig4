use std::{
    hash::{Hash, Hasher},
    sync::mpsc,
    thread,
};

// actor identifier
#[derive(Clone)]
pub struct AID<T> {
    tid: thread::ThreadId,
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
        return AID {
            tid: handle.thread().id(),
            channel: sender,
        };
    }

    pub fn send(&self, t: T) -> Result<(), mpsc::SendError<T>> {
        return self.channel.send(t);
    }
}

// two AIDs are equal iff their TIDs are equal
impl<T> PartialEq for AID<T> {
    fn eq(&self, aid: &AID<T>) -> bool {
        return self.tid == aid.tid;
    }
}
impl<T> Eq for AID<T> {}

// hash AID to same value as its TID
impl<T> Hash for AID<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.tid.hash(state);
    }
}
