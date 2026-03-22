import type { RuntimeExecutionEvent, ThinkingStage } from './types';

export function normalizeExecutionEvent(payload: unknown): RuntimeExecutionEvent | null {
  if (!isRecord(payload)) {
    return null;
  }

  if (typeof payload.type === 'string') {
    return payload as RuntimeExecutionEvent;
  }

  if (isRecord(payload.Start) && typeof payload.Start.message_id === 'string') {
    return { type: 'Start', message_id: payload.Start.message_id };
  }

  if (isRecord(payload.Chunk) && typeof payload.Chunk.content === 'string') {
    return { type: 'Chunk', content: payload.Chunk.content };
  }

  if ('Complete' in payload) {
    return { type: 'Complete' };
  }

  if (isRecord(payload.Error) && typeof payload.Error.message === 'string') {
    return { type: 'Error', message: payload.Error.message };
  }

  if (
    isRecord(payload.ThinkingStatus) &&
    typeof payload.ThinkingStatus.stage === 'string' &&
    typeof payload.ThinkingStatus.description === 'string'
  ) {
    return {
      type: 'ThinkingStatus',
      stage: payload.ThinkingStatus.stage,
      description: payload.ThinkingStatus.description,
    };
  }

  if (
    isRecord(payload.ToolCall) &&
    typeof payload.ToolCall.tool_name === 'string' &&
    payload.ToolCall.input !== undefined
  ) {
    return {
      type: 'ToolCall',
      tool_name: payload.ToolCall.tool_name,
      input: payload.ToolCall.input,
    };
  }

  if (
    isRecord(payload.ToolResult) &&
    typeof payload.ToolResult.tool_name === 'string' &&
    typeof payload.ToolResult.output === 'string' &&
    typeof payload.ToolResult.success === 'boolean'
  ) {
    return {
      type: 'ToolResult',
      tool_name: payload.ToolResult.tool_name,
      output: payload.ToolResult.output,
      success: payload.ToolResult.success,
    };
  }

  if (
    isRecord(payload.SearchSummary) &&
    Array.isArray(payload.SearchSummary.sources) &&
    typeof payload.SearchSummary.evidence_count === 'number'
  ) {
    return {
      type: 'SearchSummary',
      sources: payload.SearchSummary.sources.filter(
        (item): item is string => typeof item === 'string',
      ),
      evidence_count: payload.SearchSummary.evidence_count,
    };
  }

  if (isRecord(payload.Reasoning) && typeof payload.Reasoning.summary === 'string') {
    return {
      type: 'Reasoning',
      summary: payload.Reasoning.summary,
    };
  }

  return null;
}

export function toThinkingStage(stage: string): ThinkingStage {
  if (
    stage === 'searching' ||
    stage === 'reasoning' ||
    stage === 'tool_calling' ||
    stage === 'generating' ||
    stage === 'search_failed' ||
    stage === 'complete'
  ) {
    return stage;
  }
  return 'generating';
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}
