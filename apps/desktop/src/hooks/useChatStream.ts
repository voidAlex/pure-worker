/**
 * 流式聊天 Hook
 *
 * 管理聊天状态、流式响应、事件监听
 */

import { useReducer, useCallback, useRef, useEffect } from 'react';
import { listen, Event } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { listConversationMessages, MessageFilters, MessageListItem } from '@/services/chatService';

export interface ChatStreamInput {
  conversation_id?: string;
  message: string;
  agent_role: string;
}

/**
 * 流式聊天事件类型（与后端 ChatStreamEvent 保持同步）
 */
export type ChatStreamEvent =
  | { type: 'Start'; message_id: string }
  | { type: 'Chunk'; content: string }
  | { type: 'Complete' }
  | { type: 'Error'; message: string }
  | { type: 'ThinkingStatus'; stage: string; description: string }
  | { type: 'ToolCall'; tool_name: string; input: unknown }
  | { type: 'ToolResult'; tool_name: string; output: string; success: boolean }
  | { type: 'SearchSummary'; sources: string[]; evidence_count: number }
  | { type: 'Reasoning'; summary: string };

/**
 * 思考状态阶段枚举
 */
export type ThinkingStage = 'searching' | 'reasoning' | 'tool_calling' | 'generating';

/**
 * 思考轨迹信息
 */
export interface ThinkingTrace {
  stage: ThinkingStage;
  description: string;
  toolCalls: ToolCallInfo[];
  searchSummary?: SearchSummaryInfo;
  reasoning?: string;
}

/**
 * 工具调用信息
 */
export interface ToolCallInfo {
  toolName: string;
  input?: unknown;
  output?: string;
  success?: boolean;
}

/**
 * 搜索摘要信息
 */
export interface SearchSummaryInfo {
  sources: string[];
  evidenceCount: number;
}

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  tool_name?: string;
  created_at: string;
  isStreaming?: boolean;
  /** 思考轨迹信息（仅 assistant 消息） */
  thinkingTrace?: ThinkingTrace;
}

export interface UseChatStreamOptions {
  conversationId?: string;
  agentRole?: string;
  onError?: (error: string) => void;
}

export interface UseChatStreamReturn {
  messages: ChatMessage[];
  isStreaming: boolean;
  error: string | null;
  sendMessage: (message: string) => Promise<void>;
  clearError: () => void;
  reset: () => void;
  loadMessages: (conversationId: string) => Promise<void>;
}

interface ChatState {
  messages: ChatMessage[];
  isStreaming: boolean;
  error: string | null;
  currentConversationId: string | undefined;
}

type ChatAction =
  | { type: 'RESET'; conversationId: string | undefined }
  | { type: 'START'; messageId: string }
  | { type: 'CHUNK'; content: string }
  | { type: 'COMPLETE' }
  | { type: 'ERROR'; message: string }
  | { type: 'CLEAR_ERROR' }
  | { type: 'SET_MESSAGES'; messages: ChatMessage[] }
  | { type: 'SET_CONVERSATION_ID'; id: string }
  | { type: 'THINKING_STATUS'; stage: ThinkingStage; description: string }
  | { type: 'TOOL_CALL'; toolName: string; input: unknown }
  | { type: 'TOOL_RESULT'; toolName: string; output: string; success: boolean }
  | { type: 'SEARCH_SUMMARY'; sources: string[]; evidenceCount: number }
  | { type: 'REASONING'; summary: string };

const initialState = (conversationId: string | undefined): ChatState => ({
  messages: [],
  isStreaming: false,
  error: null,
  currentConversationId: conversationId,
});

function chatReducer(state: ChatState, action: ChatAction): ChatState {
  switch (action.type) {
    case 'RESET':
      return initialState(action.conversationId);
    case 'START':
      return {
        ...state,
        isStreaming: true,
        error: null,
        messages: [
          ...state.messages,
          {
            id: action.messageId,
            role: 'assistant',
            content: '',
            created_at: new Date().toISOString(),
            isStreaming: true,
            thinkingTrace: {
              stage: 'searching' as ThinkingStage,
              description: '',
              toolCalls: [],
            },
          },
        ],
      };
    case 'CHUNK': {
      const lastMessage = state.messages[state.messages.length - 1];
      if (lastMessage && lastMessage.isStreaming) {
        return {
          ...state,
          messages: [
            ...state.messages.slice(0, -1),
            { ...lastMessage, content: lastMessage.content + action.content },
          ],
        };
      }
      return state;
    }
    case 'COMPLETE': {
      const lastMessage = state.messages[state.messages.length - 1];
      if (lastMessage && lastMessage.isStreaming) {
        return {
          ...state,
          isStreaming: false,
          messages: [
            ...state.messages.slice(0, -1),
            { ...lastMessage, isStreaming: false },
          ],
        };
      }
      return { ...state, isStreaming: false };
    }
    case 'ERROR':
      return {
        ...state,
        isStreaming: false,
        error: action.message,
      };
    case 'CLEAR_ERROR':
      return { ...state, error: null };
    case 'SET_MESSAGES':
      return { ...state, messages: action.messages };
    case 'SET_CONVERSATION_ID':
      return { ...state, currentConversationId: action.id };
    // 处理思考状态更新
    case 'THINKING_STATUS': {
      const lastMessage = state.messages[state.messages.length - 1];
      if (lastMessage && lastMessage.isStreaming && lastMessage.thinkingTrace) {
        return {
          ...state,
          messages: [
            ...state.messages.slice(0, -1),
            {
              ...lastMessage,
              thinkingTrace: {
                ...lastMessage.thinkingTrace,
                stage: action.stage,
                description: action.description,
              },
            },
          ],
        };
      }
      return state;
    }
    // 处理工具调用
    case 'TOOL_CALL': {
      const lastMessage = state.messages[state.messages.length - 1];
      if (lastMessage && lastMessage.isStreaming && lastMessage.thinkingTrace) {
        return {
          ...state,
          messages: [
            ...state.messages.slice(0, -1),
            {
              ...lastMessage,
              thinkingTrace: {
                ...lastMessage.thinkingTrace,
                stage: 'tool_calling',
                toolCalls: [
                  ...lastMessage.thinkingTrace.toolCalls,
                  { toolName: action.toolName, input: action.input },
                ],
              },
            },
          ],
        };
      }
      return state;
    }
    // 处理工具调用结果
    case 'TOOL_RESULT': {
      const lastMessage = state.messages[state.messages.length - 1];
      if (lastMessage && lastMessage.isStreaming && lastMessage.thinkingTrace) {
        const toolCalls = [...lastMessage.thinkingTrace.toolCalls];
        // 查找对应的工具调用并更新结果
        const toolIndex = toolCalls.findIndex((tc) => tc.toolName === action.toolName && tc.output === undefined);
        if (toolIndex >= 0) {
          toolCalls[toolIndex] = {
            ...toolCalls[toolIndex],
            output: action.output,
            success: action.success,
          };
        }
        return {
          ...state,
          messages: [
            ...state.messages.slice(0, -1),
            {
              ...lastMessage,
              thinkingTrace: {
                ...lastMessage.thinkingTrace,
                toolCalls,
              },
            },
          ],
        };
      }
      return state;
    }
    // 处理搜索摘要
    case 'SEARCH_SUMMARY': {
      const lastMessage = state.messages[state.messages.length - 1];
      if (lastMessage && lastMessage.isStreaming && lastMessage.thinkingTrace) {
        return {
          ...state,
          messages: [
            ...state.messages.slice(0, -1),
            {
              ...lastMessage,
              thinkingTrace: {
                ...lastMessage.thinkingTrace,
                searchSummary: {
                  sources: action.sources,
                  evidenceCount: action.evidenceCount,
                },
              },
            },
          ],
        };
      }
      return state;
    }
    // 处理推理摘要
    case 'REASONING': {
      const lastMessage = state.messages[state.messages.length - 1];
      if (lastMessage && lastMessage.isStreaming && lastMessage.thinkingTrace) {
        return {
          ...state,
          messages: [
            ...state.messages.slice(0, -1),
            {
              ...lastMessage,
              thinkingTrace: {
                ...lastMessage.thinkingTrace,
                stage: 'reasoning',
                reasoning: action.summary,
              },
            },
          ],
        };
      }
      return state;
    }
    default:
      return state;
  }
}

export function useChatStream(options: UseChatStreamOptions = {}): UseChatStreamReturn {
  const { conversationId: initialConversationId, agentRole = 'homeroom', onError } = options;

  const [state, dispatch] = useReducer(chatReducer, initialState(initialConversationId));
  const { messages, isStreaming, error, currentConversationId } = state;

  const streamingMessageIdRef = useRef<string | null>(null);
  const unlistenRef = useRef<(() => void) | null>(null);
  const prevConversationIdRef = useRef<string | undefined>(initialConversationId);

  // 当外部 conversationId 变化时重置状态
  useEffect(() => {
    if (initialConversationId !== prevConversationIdRef.current) {
      prevConversationIdRef.current = initialConversationId;
      dispatch({ type: 'RESET', conversationId: initialConversationId });
    }
  }, [initialConversationId]);

  // 设置事件监听
  useEffect(() => {
    let isActive = true;

    const setupListener = async () => {
      const unlisten = await listen<ChatStreamEvent>(
        'chat-stream',
        (event: Event<ChatStreamEvent>) => {
          if (!isActive) return;

          const payload = event.payload;

          switch (payload.type) {
            case 'Start':
              streamingMessageIdRef.current = payload.message_id;
              dispatch({ type: 'START', messageId: payload.message_id });
              break;

            case 'Chunk':
              dispatch({ type: 'CHUNK', content: payload.content });
              break;

            case 'Complete':
              streamingMessageIdRef.current = null;
              dispatch({ type: 'COMPLETE' });
              break;

            case 'Error':
              streamingMessageIdRef.current = null;
              if (onError) onError(payload.message);
              dispatch({ type: 'ERROR', message: payload.message });
              break;

            // 处理思考状态更新
            case 'ThinkingStatus':
              dispatch({
                type: 'THINKING_STATUS',
                stage: payload.stage as ThinkingStage,
                description: payload.description,
              });
              break;

            // 处理工具调用
            case 'ToolCall':
              dispatch({
                type: 'TOOL_CALL',
                toolName: payload.tool_name,
                input: payload.input,
              });
              break;

            // 处理工具调用结果
            case 'ToolResult':
              dispatch({
                type: 'TOOL_RESULT',
                toolName: payload.tool_name,
                output: payload.output,
                success: payload.success,
              });
              break;

            // 处理搜索摘要
            case 'SearchSummary':
              dispatch({
                type: 'SEARCH_SUMMARY',
                sources: payload.sources,
                evidenceCount: payload.evidence_count,
              });
              break;

            // 处理推理摘要
            case 'Reasoning':
              dispatch({
                type: 'REASONING',
                summary: payload.summary,
              });
              break;
          }
        },
      );

      unlistenRef.current = unlisten;
    };

    setupListener();

    return () => {
      isActive = false;
      if (unlistenRef.current) {
        unlistenRef.current();
      }
    };
  }, [onError]);

  // 发送消息
  const sendMessage = useCallback(
    async (message: string) => {
      if (!message.trim() || isStreaming) return;

      const userMessageId = `user-${Date.now()}`;

      dispatch({
        type: 'START',
        messageId: userMessageId,
      });

      try {
        const input: ChatStreamInput = {
          conversation_id: currentConversationId,
          message: message.trim(),
          agent_role: agentRole,
        };

        const result = await invoke<string>('chat_stream', { input });

        if (!currentConversationId && result) {
          dispatch({ type: 'SET_CONVERSATION_ID', id: result });
        }
      } catch (e) {
        const errorMessage = e instanceof Error ? e.message : '发送消息失败';
        if (onError) onError(errorMessage);
        dispatch({ type: 'ERROR', message: errorMessage });
      }
    },
    [currentConversationId, agentRole, isStreaming, onError],
  );

  const clearError = useCallback(() => {
    dispatch({ type: 'CLEAR_ERROR' });
  }, []);

  const reset = useCallback(() => {
    dispatch({ type: 'RESET', conversationId: undefined });
  }, []);

  /**
   * 加载指定会话的历史消息
   * @param conversationId 会话ID
   */
  const loadMessages = useCallback(async (conversationId: string) => {
    try {
      const filters: MessageFilters = {
        conversationId,
        limit: 100,
      };
      const result = await listConversationMessages(filters);

      // 将服务层消息转换为 ChatMessage 格式
      const historyMessages: ChatMessage[] = result.map((msg: MessageListItem) => ({
        id: msg.id,
        role: msg.role,
        content: msg.content,
        tool_name: msg.tool_name,
        created_at: msg.created_at,
        isStreaming: false,
      }));

      dispatch({ type: 'SET_MESSAGES', messages: historyMessages });
    } catch (e) {
      const errorMessage = e instanceof Error ? e.message : '加载历史消息失败';
      if (onError) onError(errorMessage);
      dispatch({ type: 'ERROR', message: errorMessage });
    }
  }, [onError]);

  return {
    messages,
    isStreaming,
    error,
    sendMessage,
    clearError,
    reset,
    loadMessages,
  };
}
