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

export type ChatStreamEvent =
  | { type: 'Start'; message_id: string }
  | { type: 'Chunk'; content: string }
  | { type: 'Complete' }
  | { type: 'Error'; message: string };

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  tool_name?: string;
  created_at: string;
  isStreaming?: boolean;
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
  | { type: 'SET_CONVERSATION_ID'; id: string };

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
