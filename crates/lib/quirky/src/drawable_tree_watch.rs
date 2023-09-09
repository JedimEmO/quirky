use crate::drawables::Drawable;
use futures::stream::FuturesUnordered;
use futures::SinkExt;
use futures::{select, Stream, StreamExt};
use futures_signals::signal::SignalExt;
use futures_signals::signal_vec::{MutableVec, SignalVecExt};
use std::future::Future;
use std::pin::Pin;

#[async_recursion::async_recursion]
async fn drawable_tree_watch_inner(
    drawables: MutableVec<Drawable>,
    mut tx: futures::channel::mpsc::Sender<()>,
) {
    let mut drawables_stream = drawables
        .signal_vec_cloned()
        .to_signal_cloned()
        .to_stream()
        .fuse();
    let mut futures = FuturesUnordered::new();

    loop {
        let mut next_drawables = drawables_stream.select_next_some();
        let mut next_unordered = futures.select_next_some();

        select! {
                drawables = next_drawables => {
                    tx.send(()).await.expect("failed to send drawables notification");
                    futures = FuturesUnordered::new();

                   for drawable in drawables {
                        match drawable {
                            Drawable::SubTree{children, ..} => {
                                futures.push(drawable_tree_watch_inner(children.clone(), tx.clone()));
                            }
                            _ => {}
                        }
                    }
                }
                _ = next_unordered => {}
        }
    }
}

pub fn drawable_tree_watch(
    widgets: MutableVec<Drawable>,
) -> (Pin<Box<impl Stream<Item = ()>>>, impl Future<Output = ()>) {
    let (tx, rx) = futures::channel::mpsc::channel(100);

    let fut = drawable_tree_watch_inner(widgets, tx);

    let rx = Box::pin(
        futures_signals::signal::from_stream(rx)
            .map(|_| ())
            .to_stream(),
    );

    (rx, fut)
}
