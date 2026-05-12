use std::{
    fmt,
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

pub type AIDHandle = thread::JoinHandle<()>;

impl<T> AID<T> {
    // creates a channel and spawns a thread running f
    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce(AID<T>, mpsc::Receiver<T>),
        F: Send + 'static,
        T: Send + 'static,
    {
        return Self::new_joinable(f).0;
    }

    // creates a channel and spawns a thread running f
    // additionally returns the join handle of the new thread
    pub fn new_joinable<F>(f: F) -> (Self, AIDHandle)
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
        let aid = AID {
            tid: handle.thread().id(),
            channel: sender,
        };
        return (aid, handle);
    }

    // creates a channel and runs f in the current thread
    pub fn new_threadless<F>(f: F) -> Self
    where
        F: FnOnce(AID<T>, mpsc::Receiver<T>),
        F: Send + 'static,
        T: Send + 'static,
    {
        let (sender, reciever) = mpsc::channel();
        let tid = thread::current().id();
        let aid = AID {
            tid,
            channel: sender.clone(),
        };
        f(aid, reciever);
        return AID {
            tid,
            channel: sender,
        };
    }

    #[cfg(test)]
    pub fn mock() -> (Self, mpsc::Receiver<T>) {
        //Creates an AID with random tid but no starts no new thread
        //Returns both AID and mailbox in a tuple
        //Useful to write unit tests

        let (sender, reciever) = mpsc::channel();
        let handle = thread::spawn(|| ());
        let tid = handle.thread().id();
        let _ = handle.join();
        return (
            AID {
                tid: tid,
                channel: sender,
            },
            reciever,
        );
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

// print AID: just use tid
impl<T> fmt::Display for AID<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return write!(f, "AID({0:?})", self.tid);
    }
}
