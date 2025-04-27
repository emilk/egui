Example running an eframe application on an external eventloop on top of a tokio executor on Linux.

By running the event loop, eframe, and tokio in the same thread, one can leverage local async tasks.
These tasks can share data with the UI without the need for locks or message passing.

In tokio CPU-bound async tasks can be run with `spawn_blocking` to avoid impacting the UI frame rate.

```sh
cargo run -p external_eventloop_async --features linux-example
```
