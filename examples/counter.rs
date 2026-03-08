use stagecraft::{Actor, Context, Handle, HasMailbox};
use tokio_util::sync::CancellationToken;

// Receives lines and prints them. Spawned as a child of Counter.
struct Printer;

#[stagecraft::message(Printer)]
pub enum PrinterMessage {
    Print { line: String },
}

impl HasMailbox for Printer {
    type Message = PrinterMessage;
}

impl Actor for Printer {
    type Init = ();

    async fn init(_: (), _ctx: &mut Context<Self>) -> Self {
        Printer
    }

    async fn handle_message(&mut self, msg: PrinterMessage, _ctx: &mut Context<Self>) {
        match msg {
            PrinterMessage::Print { line } => println!("{line}"),
        }
    }
}

// Tracks a count. Spawns a Printer child during init and uses it to log
// each increment.

struct Counter {
    count: u64,
    printer: Handle<Printer>,
}

// Plain variants use cast (fire-and-forget); #[call(T)] uses request/reply.
#[stagecraft::message(Counter)]
pub enum CounterMessage {
    Increment,
    #[call(u64)]
    Get,
    Stop,
}

impl HasMailbox for Counter {
    type Message = CounterMessage;
}

impl Actor for Counter {
    type Init = u64;

    async fn init(start: u64, ctx: &mut Context<Self>) -> Self {
        // Child actors share the parent's cancellation token and task tracker.
        let printer = ctx.spawn::<Printer>(());
        Counter {
            count: start,
            printer,
        }
    }

    async fn handle_message(&mut self, msg: CounterMessage, ctx: &mut Context<Self>) {
        match msg {
            CounterMessage::Increment => {
                self.count += 1;
                let _ = self
                    .printer
                    .print(format!("count is now {}", self.count))
                    .await;
            }
            CounterMessage::Get { respond_to } => {
                let _ = respond_to.send(self.count);
            }
            CounterMessage::Stop => ctx.shutdown(),
        }
    }

    async fn on_stop(&mut self, _ctx: &mut Context<Self>) {
        println!("stopped at {}", self.count);
    }
}

#[tokio::main]
async fn main() {
    let token = CancellationToken::new();
    let counter = stagecraft::spawn::<Counter>(token.clone(), 0);

    counter.increment().await.unwrap();
    counter.increment().await.unwrap();
    counter.increment().await.unwrap();

    let n = counter.get().await.unwrap();
    assert_eq!(n, 3);

    // Stop triggers ctx.shutdown() inside the actor, which cancels the token
    // and causes the await below to resolve.
    counter.stop().await.unwrap();
    token.cancelled().await;
}
