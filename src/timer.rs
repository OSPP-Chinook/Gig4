use std::{marker::PhantomData, thread, time::Duration};

use crate::aid::AID;

pub struct Timer<T> {
    _phantom: PhantomData<T>,
    aid: AID<Duration>,
}

impl<T: Send + 'static + Clone> Timer<T> {
    pub fn new(owner: AID<T>, message: T) -> Self {
        return Timer::<T> {
            _phantom: PhantomData,
            aid: AID::new(move |aid, mailbox| {
                drop(aid);
                // doesn't need a KillYouself or zombie because it only communicates
                // with its owner and automatically dies once the owner drops the refernce
                for msg in mailbox {
                    thread::sleep(msg);
                    let _ = owner.send(message.clone());
                }
            }),
        };
    }

    pub fn start_timer(&self, time: Duration) {
        let _ = self.aid.send(time);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::TryRecvError;

    use super::*;

    #[test]
    fn responds() {
        let (mock, mailbox) = AID::mock();
        let timer = Timer::new(mock, "test");
        timer.start_timer(Duration::ZERO);
        assert!(matches!(mailbox.recv(), Ok("test")));
    }

    #[test]
    fn responds_in_time() {
        let (mock, mailbox) = AID::mock();
        let timer = Timer::new(mock, "test");
        timer.start_timer(Duration::from_millis(500));
        thread::sleep(Duration::from_millis(600));
        assert!(matches!(mailbox.try_recv(), Ok("test")));
    }

    #[test]
    fn responds_not_too_early() {
        let (mock, mailbox) = AID::mock();
        let timer = Timer::new(mock, "test");
        timer.start_timer(Duration::from_millis(600));
        thread::sleep(Duration::from_millis(500));
        assert!(matches!(mailbox.try_recv(), Err(TryRecvError::Empty)));
    }
}
