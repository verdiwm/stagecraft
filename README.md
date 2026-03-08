# stagecraft

A minimal Tokio-based actor framework.

## Architecture

Each actor is an isolated unit of state that communicates exclusively through
typed message channels. The runtime model has three core types:

- **`HasMailbox`**: base trait every actor implements. Declares the message
  type and inbox capacity.
- **`Handle<A>`**: the only way to reach a running actor from outside. Wraps
  an `mpsc` sender. Clone freely; dropping all handles closes the inbox and
  exits the actor.
- **`Context<A>`**: passed into every actor method. Provides the actor's own
  handle, shutdown control, and helpers for spawning children.

Shutdown is coordinated via `CancellationToken`. Each actor holds a token;
child actors receive child tokens so cancelling a parent cascades to the entire
subtree. When an actor exits it cancels its token, drains its `TaskTracker`
(waiting for any fire-and-forget futures), then calls `on_stop`.

### Actor variants

| Trait              | `Send` required | Event stream |
| ------------------ | --------------- | ------------ |
| `Actor`            | yes             | no           |
| `StreamActor`      | yes             | yes          |
| `LocalActor`       | no              | no           |
| `LocalStreamActor` | no              | yes          |

Use `Actor` for the common case. Reach for `LocalActor` when state is `!Send`
(e.g. a libinput context). The `Stream` variants add a second select arm that
drives an arbitrary `futures::Stream` alongside the message inbox.

## Example

See [`examples/counter.rs`](examples/counter.rs) for a self-contained walkthrough
covering message definition, the `#[message]` macro, actor implementation, and spawning.

```
cargo run --example counter
```

## License

This project is licensed under the
[Apache-2.0 License](http://www.apache.org/licenses/LICENSE-2.0). For more
information, please see the [LICENSE](LICENSE) file.
