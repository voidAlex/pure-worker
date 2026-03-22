export type RuntimeExecutionEvent =
  | { type: 'Start'; message_id: string }
  | { type: 'Chunk'; content: string }
  | { type: 'Complete' }
  | { type: 'Error'; message: string }
  | { type: 'ThinkingStatus'; stage: string; description: string }
  | { type: 'ToolCall'; tool_name: string; input: unknown }
  | { type: 'ToolResult'; tool_name: string; output: string; success: boolean }
  | { type: 'SearchSummary'; sources: string[]; evidence_count: number }
  | { type: 'Reasoning'; summary: string };

export type ThinkingStage =
  | 'searching'
  | 'reasoning'
  | 'tool_calling'
  | 'generating'
  | 'search_failed'
  | 'complete';

export interface ToolCallInfo {
  toolName: string;
  input?: unknown;
  output?: string;
  success?: boolean;
}

export interface SearchSummaryInfo {
  sources: string[];
  evidenceCount: number;
}

export interface ThinkingTrace {
  stage: ThinkingStage;
  description: string;
  toolCalls: ToolCallInfo[];
  searchSummary?: SearchSummaryInfo;
  reasoning?: string;
}

export interface ChatMessageItem {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  tool_name?: string;
  created_at: string;
  isStreaming?: boolean;
  thinkingTrace?: ThinkingTrace;
}
