use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use rig_agent::agent::{
    AgentHook, CompletionCallAction, CompletionCallEvent, HookContext, ModelTurnAction,
    ModelTurnFinished, RequestPatch, ToolCall, ToolCallAction, ToolResultAction, ToolResultEvent,
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
    async fn on_completion_call(
        &self,
        ctx: &HookContext,
        event: CompletionCallEvent<'_>,
    ) -> CompletionCallAction {
        ctx.scratchpad()
            .update::<Timings, _>(|timings| timings.model_started = Some(Instant::now()));
        let max_tokens = self.cap.load(Ordering::Relaxed);
        tracing::info!(run_id = ?ctx.run_id(), turn = event.turn, max_tokens, "support model call started");
        CompletionCallAction::patch(RequestPatch::new().max_tokens(max_tokens))
    }

    async fn on_model_turn_finished(
        &self,
        ctx: &HookContext,
        event: ModelTurnFinished<'_>,
    ) -> ModelTurnAction {
        let elapsed = ctx.scratchpad().update::<Timings, _>(|timings| {
            timings.model_started.take().map(|start| start.elapsed())
        });
        let action = self.truncation_action(event);
        tracing::info!(run_id = ?ctx.run_id(), turn = event.turn, elapsed_ms = elapsed.map(|elapsed| elapsed.as_millis()),
            finish_reason = ?event.finish_reason, input = event.usage.input_tokens, output = event.usage.output_tokens,
            action = ?action, "support model call finished");
        action
    }

    async fn on_tool_call(&self, ctx: &HookContext, event: ToolCall<'_>) -> ToolCallAction {
        ctx.scratchpad().update::<Timings, _>(|timings| {
            timings
                .tools
                .insert(event.internal_call_id.to_owned(), Instant::now());
        });
        if let Some(progress) = &self.progress {
            progress.send_replace(GenerationProgress::Searching);
        }
        tracing::info!(run_id = ?ctx.run_id(), tool = event.tool_name, call_id = event.internal_call_id, "support tool started");
        ToolCallAction::Run
    }

    async fn on_tool_result(
        &self,
        ctx: &HookContext,
        event: ToolResultEvent<'_>,
    ) -> ToolResultAction {
        let (elapsed, active) = ctx.scratchpad().update::<Timings, _>(|timings| {
            (
                timings
                    .tools
                    .remove(event.internal_call_id)
                    .map(|start| start.elapsed()),
                !timings.tools.is_empty(),
            )
        });
        if !active && let Some(progress) = &self.progress {
            progress.send_replace(GenerationProgress::Thinking);
        }
        tracing::info!(run_id = ?ctx.run_id(), tool = event.tool_name, call_id = event.internal_call_id,
            elapsed_ms = elapsed.map(|elapsed| elapsed.as_millis()), status = event.raw_result.status_name(),
            error_kind = ?event.raw_result.error().map(rig_core::tool::ToolExecutionError::kind), "support tool finished");
        ToolResultAction::Keep
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
        let model = MockCompletionModel::new([
            MockTurn::text("partial").with_finish_reason(FinishReason::Length),
            MockTurn::text("complete").with_finish_reason(FinishReason::Stop),
        ]);
        AgentBuilder::new(model.clone())
            .add_hook(SupportHooks::new(100, RetryMode::Response, None))
            .build()
            .runner("question")
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
        let model = MockCompletionModel::new([
            MockTurn::text("partial").with_finish_reason(FinishReason::Length),
            MockTurn::text("partial again").with_finish_reason(FinishReason::Length),
        ]);
        let result = AgentBuilder::new(model.clone())
            .add_hook(SupportHooks::new(100, RetryMode::Response, None))
            .build()
            .runner("question")
            .max_turns(MAX_RESPONSE_CALLS)
            .run()
            .await;
        ensure!(result.is_err());
        ensure!(model.request_count() == 2);
        Ok(())
    }

    #[tokio::test]
    async fn does_not_retry_filter_stops_or_missing_finish_metadata() -> Result<()> {
        for turn in [
            MockTurn::text("filtered").with_finish_reason(FinishReason::ContentFilter),
            MockTurn::text("answer"),
        ] {
            let model = MockCompletionModel::new([turn]);
            AgentBuilder::new(model.clone())
                .add_hook(SupportHooks::new(100, RetryMode::Response, None))
                .build()
                .runner("question")
                .max_turns(MAX_RESPONSE_CALLS)
                .run()
                .await?;
            ensure!(model.request_count() == 1);
        }
        Ok(())
    }

    struct ObserveProgress(watch::Receiver<GenerationProgress>);

    impl AgentHook for ObserveProgress {
        async fn on_tool_call(&self, _: &HookContext, _: ToolCall<'_>) -> ToolCallAction {
            assert_eq!(*self.0.borrow(), GenerationProgress::Searching);
            ToolCallAction::Run
        }

        async fn on_tool_result(
            &self,
            _: &HookContext,
            event: ToolResultEvent<'_>,
        ) -> ToolResultAction {
            assert!(event.raw_result.is_error());
            assert_eq!(*self.0.borrow(), GenerationProgress::Thinking);
            ToolResultAction::Keep
        }
    }

    #[tokio::test]
    async fn reports_tool_progress_and_resets_it_after_failure_without_replaying_tools()
    -> Result<()> {
        let (tx, rx) = watch::channel(GenerationProgress::Thinking);
        let model = MockCompletionModel::new([
            MockTurn::tool_call("call", "flaky_tool", serde_json::json!({}))
                .with_finish_reason(FinishReason::Length),
            MockTurn::text("Unable to search"),
        ]);
        AgentBuilder::new(model.clone())
            .tool(MockFailingTool::new(ToolErrorKind::RateLimited))
            .add_hook(SupportHooks::new(100, RetryMode::Response, Some(tx)))
            .add_hook(ObserveProgress(rx.clone()))
            .build()
            .runner("question")
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
