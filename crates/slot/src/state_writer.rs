use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use slot_store::{write_shelf_carts, write_slot_state, Recent, SlotState};

use crate::card_log::CardWrites;

#[derive(Default)]
struct Queue {
    next: Option<(PathBuf, SlotState)>,
    shelf_carts: Option<(PathBuf, Vec<(String, Recent)>)>,
    busy: bool,
    closed: bool,
}

type Shared = Arc<(Mutex<Queue>, Condvar)>;

pub struct StateWriter {
    shared: Shared,
    threaded: bool,
    failing: Arc<CardWrites>,
}

impl StateWriter {
    pub fn spawn() -> Self {
        let shared: Shared = Arc::default();
        let failing: Arc<CardWrites> = Arc::default();
        let worker = shared.clone();
        let flag = failing.clone();
        let threaded = thread::Builder::new()
            .name("slot-state".into())
            .spawn(move || work(&worker, &flag))
            .is_ok();
        StateWriter {
            shared,
            threaded,
            failing,
        }
    }

    pub fn save(&self, root: &Path, state: &SlotState) {
        if !self.threaded {
            return write(root, state, &self.failing);
        }
        let (lock, wake) = &*self.shared;
        let mut q = lock.lock().unwrap_or_else(|e| e.into_inner());
        q.next = Some((root.to_path_buf(), state.clone()));
        wake.notify_all();
    }

    pub fn save_shelf_carts(&self, root: &Path, carts: &[(String, Recent)]) {
        if !self.threaded {
            return write_carts(root, carts, &self.failing);
        }
        let (lock, wake) = &*self.shared;
        let mut q = lock.lock().unwrap_or_else(|e| e.into_inner());
        q.shelf_carts = Some((root.to_path_buf(), carts.to_vec()));
        wake.notify_all();
    }

    pub fn card_writes(&self) -> Arc<CardWrites> {
        self.failing.clone()
    }

    pub fn flush(&self) {
        let (lock, wake) = &*self.shared;
        let mut q = lock.lock().unwrap_or_else(|e| e.into_inner());
        while q.next.is_some() || q.shelf_carts.is_some() || q.busy {
            q = wake.wait(q).unwrap_or_else(|e| e.into_inner());
        }
    }
}

impl Drop for StateWriter {
    fn drop(&mut self) {
        self.flush();
        let (lock, wake) = &*self.shared;
        lock.lock().unwrap_or_else(|e| e.into_inner()).closed = true;
        wake.notify_all();
    }
}

fn work(shared: &Shared, failing: &CardWrites) {
    let (lock, wake) = &**shared;
    loop {
        let (state, carts) = {
            let mut q = lock.lock().unwrap_or_else(|e| e.into_inner());
            loop {
                if q.next.is_some() || q.shelf_carts.is_some() {
                    q.busy = true;
                    break (q.next.take(), q.shelf_carts.take());
                }
                if q.closed {
                    return;
                }
                q = wake.wait(q).unwrap_or_else(|e| e.into_inner());
            }
        };
        if let Some((root, state)) = state {
            write(&root, &state, failing);
        }
        if let Some((root, carts)) = carts {
            write_carts(&root, &carts, failing);
        }
        lock.lock().unwrap_or_else(|e| e.into_inner()).busy = false;
        wake.notify_all();
    }
}

fn write(root: &Path, state: &SlotState, failing: &CardWrites) {
    let written = write_slot_state(root, state);
    if let Err(e) = &written {
        eprintln!("slot: slot.state: {e}");
    }
    failing.wrote("slot.state", &written);
}

fn write_carts(root: &Path, carts: &[(String, Recent)], failing: &CardWrites) {
    let written = write_shelf_carts(root, carts);
    if let Err(e) = &written {
        eprintln!("slot: shelf_carts.txt: {e}");
    }
    failing.wrote("shelf_carts.txt", &written);
}
