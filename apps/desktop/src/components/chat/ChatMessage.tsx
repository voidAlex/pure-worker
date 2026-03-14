/**
 * 单条聊天消息组件
 */

import React from 'react';
import { ChatMessage as ChatMessageType } from '@/hooks/useChatStream';

export interface ChatMessageProps {
  message: ChatMessageType;
}

export function ChatMessage({ message }: ChatMessageProps) {
  const isUser = message.role === 'user';
  
  return (
    <div className={`flex ${isUser ? 'justify-end' : 'justify-start'}`}>
      <div
        className={`max-w-[80%] rounded-2xl px-4 py-3 ${
          isUser
            ? 'bg-blue-500 text-white'
            : 'bg-gray-100 text-gray-800'
        }`}
      >
        <div className={`text-xs mb-1 ${isUser ? 'text-blue-200' : 'text-gray-500'}`}>
          {isUser ? '我' : 'AI 助手'}
        </div>
        <div className="whitespace-pre-wrap leading-relaxed">
          {message.content || (message.isStreaming ? '' : '...')}
        </div>
        {message.tool_name && (
          <div className={`text-xs mt-2 ${isUser ? 'text-blue-200' : 'text-gray-500'}`}>
            使用了工具: {message.tool_name}
          </div>
        )}
      </div>
    </div>
  );
}
