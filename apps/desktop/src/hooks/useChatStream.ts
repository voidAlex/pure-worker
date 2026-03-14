/**
 * 流式聊天 Hook
 * 
 * 管理聊天状态、流式响应、事件监听
 */

import { useState, useCallback, useRef, useEffect } from 'react';
import { listen, Event } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
export interface ChatStreamInput { conversation_id?: string; message: string; agent_role: string; }

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
}

export function useChatStream(options: UseChatStreamOptions = {}): UseChatStreamReturn {
  const { conversationId: initialConversationId, agentRole = 'homeroom', onError } = options;
  
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isStreaming, setIsStreaming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [currentConversationId, setCurrentConversationId] = useState<string | undefined>(initialConversationId);
  
  const streamingMessageIdRef = useRef<string | null>(null);
  const unlistenRef = useRef<(() => void) | null>(null);

  // 设置事件监听
  useEffect(() => {
    let isActive = true;

    const setupListener = async () => {
      const unlisten = await listen<ChatStreamEvent>('chat-stream', (event: Event<ChatStreamEvent>) => {
        if (!isActive) return;
        
        const payload = event.payload;
        
        switch (payload.type) {
          case 'Start':
            setIsStreaming(true);
            setError(null);
            streamingMessageIdRef.current = payload.message_id;
            setMessages(prev => [...prev, {
              id: payload.message_id,
              role: 'assistant',
              content: '',
              created_at: new Date().toISOString(),
              isStreaming: true,
            }]);
            break;
            
          case 'Chunk':
            setMessages(prev => {
              const lastMessage = prev[prev.length - 1];
              if (lastMessage && lastMessage.isStreaming) {
                return [
                  ...prev.slice(0, -1),
                  { ...lastMessage, content: lastMessage.content + payload.content },
                ];
              }
              return prev;
            });
            break;
            
          case 'Complete':
            setIsStreaming(false);
            streamingMessageIdRef.current = null;
            setMessages(prev => {
              const lastMessage = prev[prev.length - 1];
              if (lastMessage && lastMessage.isStreaming) {
                return [...prev.slice(0, -1), { ...lastMessage, isStreaming: false }];
              }
              return prev;
            });
            break;
            
          case 'Error':
            setIsStreaming(false);
            setError(payload.message);
            streamingMessageIdRef.current = null;
            if (onError) onError(payload.message);
            break;
        }
      });
      
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
  const sendMessage = useCallback(async (message: string) => {
    if (!message.trim() || isStreaming) return;

    const userMessageId = `user-${Date.now()}`;
    setMessages(prev => [...prev, {
      id: userMessageId,
      role: 'user',
      content: message.trim(),
      created_at: new Date().toISOString(),
    }]);

    setError(null);

    try {
      const input: ChatStreamInput = {
        conversation_id: currentConversationId,
        message: message.trim(),
        agent_role: agentRole,
      };

      const result = await invoke<string>('chat_stream', { input });
      
      if (!currentConversationId && result) {
        setCurrentConversationId(result);
      }
    } catch (e) {
      const errorMessage = e instanceof Error ? e.message : '发送消息失败';
      setError(errorMessage);
      setIsStreaming(false);
      if (onError) onError(errorMessage);
    }
  }, [currentConversationId, agentRole, isStreaming, onError]);

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  return {
    messages,
    isStreaming,
    error,
    sendMessage,
    clearError,
  };
}
