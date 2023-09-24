use futures::future::BoxFuture;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, LockResult, Mutex, MutexGuard};
use std::task::{Context, Poll, Waker};
use uuid::Uuid;

#[derive(Default)]
pub struct FuturesMap<'a> {
    data: Arc<Mutex<HashMap<Uuid, BoxFuture<'a, ()>>>>,
    waker: Mutex<Option<Waker>>,
}

impl<'a> FuturesMap<'a> {
    pub fn lock(&self) -> LockResult<MutexGuard<'_, HashMap<Uuid, BoxFuture<'a, ()>>>> {
        self.data.lock()
    }

    pub fn keys(&self) -> HashSet<Uuid> {
        self.data
            .lock()
            .expect("FuturesMapPoll data lock error")
            .keys()
            .map(|k| *k)
            .collect()
    }

    pub fn contains_key(&self, key: &Uuid) -> bool {
        self.data
            .lock()
            .expect("FuturesMapPoll data lock error")
            .contains_key(key)
    }

    pub fn insert(&self, key: &Uuid, fut: BoxFuture<'a, ()>) {
        self.data
            .lock()
            .expect("FuturesMapPoll data lock error")
            .insert(*key, fut);
        self.wake();
    }

    pub fn remove(&self, key: &Uuid) -> bool {
        let mut data = self.data.lock().expect("FuturesMapPoll data lock error");

        if !data.contains_key(key) {
            return false;
        }

        data.remove(key);
        self.wake();

        return true;
    }

    fn wake(&self) {
        if let Some(waker) = self
            .waker
            .lock()
            .expect("FuturesMapPoll waker lock error")
            .take()
        {
            waker.wake();
        }
    }
}

pub struct FuturesMapPoll<'a> {
    futures_map: Arc<FuturesMap<'a>>,
}

impl<'a> FuturesMapPoll<'a> {
    pub fn new() -> (Self, Arc<FuturesMap<'a>>) {
        let data: Arc<FuturesMap<'a>> = Default::default();

        let d_out = data.clone();

        (
            Self {
                futures_map: data.clone(),
            },
            d_out,
        )
    }
}

impl<'a> Future for FuturesMapPoll<'a> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let ids_to_remove = {
            let mut data = self
                .futures_map
                .data
                .lock()
                .expect("FuturesMapPoll data lock error");
            let mut ids_to_remove = vec![];

            for fut in data.iter_mut() {
                if let Poll::Ready(_) = fut.1.as_mut().poll(cx) {
                    ids_to_remove.push(*fut.0);
                }
            }

            ids_to_remove
        };

        for to_remove in ids_to_remove {
            self.futures_map.remove(&to_remove);
        }

        let _ = self
            .futures_map
            .waker
            .lock()
            .expect("FuturesMapPoll waker lock error")
            .insert(cx.waker().clone());

        Poll::Pending
    }
}
