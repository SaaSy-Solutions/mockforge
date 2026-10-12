//! HTTP hook logging must accept response bodies with multibyte characters.

use crate::advanced_orchestration::{ExecutionContext, Hook, HookAction, HookType};
use std::{
    error::Error,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{span, Event, Metadata, Subscriber};

struct EnabledLogging(Arc<AtomicBool>);

struct SuccessVisitor<'a>(&'a AtomicBool);

impl tracing::field::Visit for SuccessVisitor<'_> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" && format!("{value:?}").contains("→ 200") {
            self.0.store(true, Ordering::Relaxed);
        }
    }
}

impl Subscriber for EnabledLogging {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
        span::Id::from_u64(1)
    }
    fn record(&self, _: &span::Id, _: &span::Record<'_>) {}
    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
    fn event(&self, event: &Event<'_>) {
        event.record(&mut SuccessVisitor(&self.0));
    }
    fn enter(&self, _: &span::Id) {}
    fn exit(&self, _: &span::Id) {}
}

#[tokio::test(flavor = "current_thread")]
async fn unicode_http_response_does_not_abort_following_hook_actions() -> Result<(), Box<dyn Error>>
{
    // The logging expression is lazy: a subscriber is necessary to exercise it.
    let logged_success = Arc::new(AtomicBool::new(false));
    let _logging = tracing::subscriber::set_default(EnabledLogging(Arc::clone(&logged_success)));
    for body in [format!("{}€ response", "a".repeat(99)), "🧪".repeat(30)] {
        logged_success.store(false, Ordering::Relaxed);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await?;
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let mut byte = [0_u8; 1];
                stream.read_exact(&mut byte).await?;
                request.push(byte[0]);
                assert!(request.len() <= 4096, "Fixture request headers exceeded their limit");
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(), body
            );
            stream.write_all(response.as_bytes()).await?;
            stream.shutdown().await
        });
        let hook = Hook {
            name: "unicode response".into(),
            hook_type: HookType::PreStep,
            condition: None,
            actions: vec![
                HookAction::HttpRequest {
                    url: format!("http://{address}/"),
                    method: "GET".into(),
                    body: None,
                },
                HookAction::SetVariable {
                    name: "continued".into(),
                    value: serde_json::json!(true),
                },
            ],
        };
        let mut context = ExecutionContext::new();
        tokio::time::timeout(std::time::Duration::from_secs(5), hook.execute(&mut context))
            .await??;
        assert!(logged_success.load(Ordering::Relaxed), "HTTP success logging was not exercised");
        assert_eq!(context.get_variable("continued")?, &serde_json::json!(true));
        server.await??;
    }
    Ok(())
}
