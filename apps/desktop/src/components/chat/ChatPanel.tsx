/**
 * 聊天面板组件
 *
 * 主聊天界面，包含消息列表和输入框
 */

import React, { useEffect, useState } from 'react';
import { useChatStream } from '@/hooks/useChatStream';
import { ConversationList } from './ConversationList';
import { ChatMessage } from './ChatMessage';
import { listConversations, ConversationListItem } from '@/services/chatService';

export interface ChatPanelProps {
  conversationId?: string;
  agentRole?: string;
  className?: string;
  teacherId: string;
}

export function ChatPanel({
  conversationId,
  agentRole = 'homeroom',
  className = '',
  teacherId,
}: ChatPanelProps) {
  const [conversations, setConversations] = useState<ConversationListItem[]>([]);
  const [currentConversationId, setCurrentConversationId] = useState<string | undefined>(
    conversationId,
  );

  const { messages, isStreaming, error, sendMessage, clearError, reset, loadMessages } = useChatStream({
    conversationId: currentConversationId,
    agentRole,
  });

  // 加载会话列表
  const loadConversations = async () => {
    try {
      const result = await listConversations({
        teacherId,
        limit: 50,
      });
      setConversations(result.conversations);
    } catch (e) {
      console.error('加载会话列表失败:', e);
    }
  };

  useEffect(() => {
    loadConversations();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [teacherId]);

  const handleSelectConversation = async (id: string) => {
    setCurrentConversationId(id);
    if (id) {
      await loadMessages(id);
    }
  };

  const handleCreateNew = () => {
    setCurrentConversationId(undefined);
    reset();
  };

  const handleSendMessage = async (message: string) => {
    await sendMessage(message);
    if (!currentConversationId) {
      await loadConversations();
    }
  };

  return (
    <div className={`flex h-full bg-white ${className}`}>
      {/* 左侧会话列表 */}
      <ConversationList
        conversations={conversations}
        currentId={currentConversationId}
        onSelect={handleSelectConversation}
        onCreateNew={handleCreateNew}
      />

      {/* 右侧聊天区域 */}
      <div className="flex-1 flex flex-col h-full">
        {/* 消息列表 */}
        <div className="flex-1 overflow-y-auto p-4 space-y-4">
          {messages.length === 0 ? (
            <div className="flex items-center justify-center h-full text-gray-400">
              <div className="text-center">
                <p className="text-lg mb-2">开始新的对话</p>
                <p className="text-sm">输入消息与 AI 助手交流</p>
              </div>
            </div>
          ) : (
            <div className="space-y-4">
              {messages.map((message, index) => (
                <ChatMessage key={message.id || index} message={message} />
              ))}
            </div>
          )}

          {/* 流式指示器 */}
          {isStreaming && (
            <div className="flex items-center gap-2 text-gray-400 text-sm">
              <div className="flex gap-1">
                <div
                  className="w-2 h-2 bg-blue-500 rounded-full animate-bounce"
                  style={{ animationDelay: '0ms' }}
                />
                <div
                  className="w-2 h-2 bg-blue-500 rounded-full animate-bounce"
                  style={{ animationDelay: '150ms' }}
                />
                <div
                  className="w-2 h-2 bg-blue-500 rounded-full animate-bounce"
                  style={{ animationDelay: '300ms' }}
                />
              </div>
              <span>AI 正在思考...</span>
            </div>
          )}
        </div>

        {/* 错误提示 */}
        {error && (
          <div className="px-4 py-2 bg-red-50 border-t border-red-100">
            <div className="flex items-center justify-between">
              <span className="text-red-600 text-sm">{error}</span>
              <button onClick={clearError} className="text-red-400 hover:text-red-600 text-sm">
                清除
              </button>
            </div>
          </div>
        )}

        {/* 输入框 */}
        <div className="border-t border-gray-200 p-4">
          <ChatInput onSend={handleSendMessage} disabled={isStreaming} />
        </div>
      </div>
    </div>
  );
}

// Inline ChatInput component for now
interface ChatInputProps {
  onSend: (message: string) => void;
  disabled?: boolean;
}

function ChatInput({ onSend, disabled = false }: ChatInputProps) {
  const [message, setMessage] = useState('');

  const handleSend = () => {
    if (message.trim() && !disabled) {
      onSend(message.trim());
      setMessage('');
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div className="flex gap-2">
      <textarea
        value={message}
        onChange={(e) => setMessage(e.target.value)}
        onKeyDown={handleKeyDown}
        disabled={disabled}
        placeholder="输入消息..."
        rows={1}
        className="flex-1 resize-none rounded-lg border border-gray-300 px-4 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:bg-gray-100"
        style={{ minHeight: '44px', maxHeight: '120px' }}
      />
      <button
        onClick={handleSend}
        disabled={disabled || !message.trim()}
        className="px-6 py-2 bg-blue-500 text-white rounded-lg font-medium disabled:bg-gray-300 hover:bg-blue-600 transition-colors"
      >
        发送
      </button>
    </div>
  );
}
