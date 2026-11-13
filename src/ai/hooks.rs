use std::{
    collections::HashMap,
    future::{Future, ready},
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use rig_agent::agent::{
    AgentHook, CompletionCallAction, CompletionCallEvent, DispatchAction, DispatchEvent,
    HookContext, ModelTurnAction, ModelTurnFinished, OutcomeAction, OutcomeEvent, RequestPatch,
};
use rig_core::{completion::FinishReason, message::AssistantContent};
use tokio::sync::watch;

pub const MAX_RESPONSE_CALLS: usize = 6;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GenerationProgress {
    #[default]
    Thinking,
    Searching,
}

#[derive(Clone, Copy)]
pub enum RetryMode {
    Response,
    Extraction,
}

pub struct SupportHooks {
    cap: AtomicU64,
    ceiling: u64,
    mode: RetryMode,
    progress: Option<watch::Sender<GenerationProgress>>,
}

#[derive(Clone, Default)]
struct Timings {
    model_started: Option<Instant>,
    tools: HashMap<String, Instant>,
}

impl SupportHooks {
    pub fn new(
        cap: u64,
        mode: RetryMode,
        progress: Option<watch::Sender<GenerationProgress>>,
    ) -> Self {
        Self {
            cap: AtomicU64::new(cap),
            ceiling: cap.saturating_mul(2),
            mode,
            progress,
        }
    }

    fn truncation_action(&self, event: ModelTurnFinished<'_>) -> ModelTurnAction {
        if !matches!(event.finish_reason, Some(FinishReason::Length)) {
            return ModelTurnAction::Continue;
        }
        // Never replay a tool-bearing response: its valid calls can still execute.
        if matches!(self.mode, RetryMode::Response)
            && event
                .content
                .iter()
                .any(|part| matches!(part, AssistantContent::ToolCall(_)))
        {
            return ModelTurnAction::Continue;
        }
        let cap = event
            .max_tokens
            .unwrap_or_else(|| self.cap.load(Ordering::Relaxed));
        if cap >= self.ceiling
            || (matches!(self.mode, RetryMode::Response) && event.turn >= MAX_RESPONSE_CALLS)
        {
            return ModelTurnAction::stop("Model output remained truncated at the retry limit");
        }
        self.cap
            .store(cap.saturating_mul(2).min(self.ceiling), Ordering::Relaxed);
        match self.mode {
            RetryMode::Response => ModelTurnAction::repeat(),
            // Extractor retries start a new runner; retain the raised cap in this hook.
            RetryMode::Extraction => ModelTurnAction::stop(
                "Structured output was truncated; retry with a larger token limit",
            ),
        }
    }
}

impl AgentHook for SupportHooks {
    fn on_completion_call(
        &self,
        ctx: &HookContext,
        event: CompletionCallEvent<'_>,
    ) -> impl Future<Output = CompletionCallAction> {
        ctx.scratchpad()
            .update::<Timings, _>(|timings| timings.model_started = Some(Instant::now()));
        let max_tokens = self.cap.load(Ordering::Relaxed);
        tracing::info!(run_id = ?ctx.run_id(), turn = event.turn, max_tokens, "support model call started");
        ready(CompletionCallAction::patch(
            RequestPatch::new().max_tokens(max_tokens),
        ))
    }

    fn on_model_turn_finished(
        &self,
        ctx: &HookContext,
        event: ModelTurnFinished<'_>,
    ) -> impl Future<Output = ModelTurnAction> {
        let elapsed = ctx.scratchpad().update::<Timings, _>(|timings| {
            timings.model_started.take().map(|start| start.elapsed())
        });
        let action = self.truncation_action(event);
        tracing::info!(run_id = ?ctx.run_id(), turn = event.turn, elapsed_ms = elapsed.map(|elapsed| elapsed.as_millis()),
            finish_reason = ?event.finish_reason, input = event.usage.input_tokens, output = event.usage.output_tokens,
            action = ?action, "support model call finished");
        ready(action)
    }

    fn on_dispatch(
        &self,
        ctx: &HookContext,
        event: DispatchEvent<'_>,
    ) -> impl Future<Output = DispatchAction> {
        let Some(tool_name) = event.tool_name() else {
            return ready(DispatchAction::Proceed);
        };
        ctx.scratchpad().update::<Timings, _>(|timings| {
            timings.tools.insert(event.id.to_string(), Instant::now());
        });
        if let Some(progress) = &self.progress {
            progress.send_replace(GenerationProgress::Searching);
        }
        tracing::info!(run_id = ?ctx.run_id(), tool = tool_name, call_id = %event.id, "support tool started");
        ready(DispatchAction::Proceed)
    }

    fn on_outcome(
        &self,
        ctx: &HookContext,
        event: OutcomeEvent<'_>,
    ) -> impl Future<Output = OutcomeAction> {
        let Some(tool_name) = event.tool_name() else {
            return ready(OutcomeAction::Proceed);
        };
        let (elapsed, active) = ctx.scratchpad().update::<Timings, _>(|timings| {
            (
                timings
                    .tools
                    .remove(&event.id.to_string())
                    .map(|start| start.elapsed()),
                !timings.tools.is_empty(),
            )
        });
        if !active && let Some(progress) = &self.progress {
            progress.send_replace(GenerationProgress::Thinking);
        }
        tracing::info!(run_id = ?ctx.run_id(), tool = tool_name, call_id = %event.id,
            elapsed_ms = elapsed.map(|elapsed| elapsed.as_millis()), status = ?event.tool_result().map(rig_core::tool::ToolResult::status_name), "support tool finished");
        ready(OutcomeAction::Proceed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Result, ensure};
    use rig_agent::{
        AgentBuilder,
        test_utils::{MockCompletionModel, MockFailingTool, MockTurn},
    };
    use rig_core::tool::ToolErrorKind;

    #[tokio::test]
    async fn retries_truncation_with_more_room_without_replaying_partial_text() -> Result<()> {
        let model = MockCompletionModel::from_turns([
            MockTurn::text("partial").with_finish_reason(FinishReason::Length),
            MockTurn::text("complete").with_finish_reason(FinishReason::Stop),
        ]);
        AgentBuilder::new(model.clone())
            .add_hook(SupportHooks::new(100, RetryMode::Response, None))
            .build()
            .prompt("question")
            .max_turns(MAX_RESPONSE_CALLS)
            .run()
            .await?;
        let requests = model.requests();
        ensure!(requests.len() == 2);
        ensure!(requests[0].max_tokens == Some(100));
        ensure!(requests[1].max_tokens == Some(200));
        ensure!(requests[0].chat_history == requests[1].chat_history);
        Ok(())
    }

    #[tokio::test]
    async fn stops_at_the_retry_ceiling() -> Result<()> {
        let model = MockCompletionModel::from_turns([
            MockTurn::text("partial").with_finish_reason(FinishReason::Length),
            MockTurn::text("partial again").with_finish_reason(FinishReason::Length),
        ]);
        let result = AgentBuilder::new(model.clone())
            .add_hook(SupportHooks::new(100, RetryMode::Response, None))
            .build()
            .prompt("question")
            .max_turns(MAX_RESPONSE_CALLS)
            .run()
            .await;
        ensure!(result.is_err());
        ensure!(model.request_count() == 2);
        Ok(())
    }

    #[tokio::test]
    async fn does_not_retry_filter_stops_or_missing_finish_metadata() -> Result<()> {
        for (turn, filtered) in [
            (
                MockTurn::text("filtered").with_finish_reason(FinishReason::ContentFilter),
                true,
            ),
            (MockTurn::text("answer"), false),
        ] {
            let model = MockCompletionModel::from_turns([turn]);
            let result = AgentBuilder::new(model.clone())
                .add_hook(SupportHooks::new(100, RetryMode::Response, None))
                .build()
                .prompt("question")
                .max_turns(MAX_RESPONSE_CALLS)
                .run()
                .await;
            if filtered {
                let error = result.expect_err("content filtering must fail the turn");
                ensure!(error.to_string().contains("content_filter"));
            } else {
                result?;
            }
            ensure!(model.request_count() == 1);
        }
        Ok(())
    }

    struct ObserveProgress(watch::Receiver<GenerationProgress>);

    impl AgentHook for ObserveProgress {
        fn on_dispatch(
            &self,
            _: &HookContext,
            event: DispatchEvent<'_>,
        ) -> impl Future<Output = DispatchAction> {
            if event.tool_name().is_none() {
                return ready(DispatchAction::Proceed);
            }
            assert_eq!(*self.0.borrow(), GenerationProgress::Searching);
            ready(DispatchAction::Proceed)
        }

        fn on_outcome(
            &self,
            _: &HookContext,
            event: OutcomeEvent<'_>,
        ) -> impl Future<Output = OutcomeAction> {
            if event.tool_name().is_none() {
                return ready(OutcomeAction::Proceed);
            }
            assert!(
                event
                    .tool_result()
                    .is_some_and(rig_core::tool::ToolResult::is_error)
            );
            assert_eq!(*self.0.borrow(), GenerationProgress::Thinking);
            ready(OutcomeAction::Proceed)
        }
    }

    #[tokio::test]
    async fn reports_tool_progress_and_resets_it_after_failure_without_replaying_tools()
    -> Result<()> {
        let (tx, rx) = watch::channel(GenerationProgress::Thinking);
        let model = MockCompletionModel::from_turns([
            MockTurn::tool_call("call", "flaky_tool", serde_json::json!({}))
                .with_finish_reason(FinishReason::Length),
            MockTurn::text("Unable to search"),
        ]);
        AgentBuilder::new(model.clone())
            .tool(MockFailingTool::new(ToolErrorKind::RateLimited))
            .add_hook(SupportHooks::new(100, RetryMode::Response, Some(tx)))
            .add_hook(ObserveProgress(rx.clone()))
            .build()
            .prompt("question")
            .max_turns(MAX_RESPONSE_CALLS)
            .run()
            .await?;
        ensure!(model.request_count() == 2);
        ensure!(
            model
                .requests()
                .iter()
                .all(|request| request.max_tokens == Some(100))
        );
        ensure!(*rx.borrow() == GenerationProgress::Thinking);
        Ok(())
    }
}
