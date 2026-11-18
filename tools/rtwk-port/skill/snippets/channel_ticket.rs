// Results from elsewhere, each with its ticket: work started on the runtime (`spawn_tokio`) and items
// pushed from any thread (`AppCx::channel`) both land on the UI thread as a closure over the model.
// The model keeps the current ticket; a result carrying an older one is dropped (R1: stale results).
// Taken from crates/rtwk/SKILL.md ("Tokio and streaming") and nav-proto (`AppView::on_start` + `cx.channel`).
use std::time::Duration;

use rtwk::app::{AppCx, AppView, ViewCx};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Sender};
use rtwk_tokio::TokioApp;

#[derive(Default)]
pub struct App {
    pub ticket: u64,           // the request that is current
    pub answer: Option<String>,
    pub chunks: Vec<String>,   // streamed pieces of the current request
    pub tx: Option<Sender<Event>>, // handed to a backend thread; set once, in `on_start`
}

/// What a backend thread sends: always with the ticket of the request it answers.
pub enum Event {
    Chunk { ticket: u64, text: String },
}

impl App {
    pub fn apply(&mut self, ev: Event) {
        match ev {
            Event::Chunk { ticket, text } if ticket == self.ticket => self.chunks.push(text),
            Event::Chunk { .. } => {} // stale: a newer request replaced it
        }
    }

    /// A new request: bump the ticket, drop what the last one left, start the work.
    pub fn ask(&mut self, cx: &mut AppCx<'_, '_, App>, question: &'static str) {
        self.ticket += 1;
        let ticket = self.ticket;
        self.chunks.clear();
        self.answer = None;
        cx.spawn_tokio(
            async move {
                tokio::time::sleep(Duration::from_millis(5)).await;
                format!("answer to {question}")
            },
            move |app, answer, _| {
                if app.ticket == ticket {
                    app.answer = Some(answer);
                } // else: stale, dropped
            },
        );
    }
}

pub fn app_view() -> AppView<App> {
    AppView::new(App::default(), view).on_start(|a, cx| {
        a.tx = Some(cx.channel(|a, ev: Event, _| a.apply(ev)));
    })
}

pub fn view(app: &App, _cx: &mut ViewCx<'_, '_, App>) -> AnyWidget {
    let text = match &app.answer {
        Some(answer) => format!("done: {answer}"),
        None => format!("waiting ({} chunks)", app.chunks.len()),
    };
    Text::new(text).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtwk_test::{Options, Tester, wait_until};

    #[test]
    fn a_stale_result_is_dropped_and_the_current_one_lands() {
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
        let _entered = rtwk_tokio::enter(&runtime);
        let mut t = Tester::with(Options::new().size(40, 3));
        t.pump_app(app_view());
        t.update_app(|a: &mut App, cx| a.ask(cx, "first"));
        t.update_app(|a: &mut App, cx| a.ask(cx, "second")); // ticket 2 replaces ticket 1
        let tx = t.read_app(|a: &App| a.tx.clone().unwrap());
        assert!(tx.send(Event::Chunk { ticket: 1, text: "old".into() }).is_ok());
        assert!(tx.send(Event::Chunk { ticket: 2, text: "new".into() }).is_ok());
        wait_until("the second answer", || {
            t.pump();
            t.read_app(|a: &App| a.answer.is_some())
        });
        t.read_app(|a: &App| {
            assert_eq!(a.answer.as_deref(), Some("answer to second"));
            assert_eq!(a.chunks, ["new"]);
        });
        t.assert_line("done: answer to second");
    }
}
