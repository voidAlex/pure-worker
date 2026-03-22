/**
 * 聊天消息列表组件
 */

import React, { useRef, useEffect } from 'react';
import { ChatMessage } from './ChatMessage';
import type { ChatMessageItem as ChatMessageType } from './types';

export interface ChatMessageListProps {
  messages: ChatMessageType[];
}

export function ChatMessageList({ messages }: ChatMessageListProps) {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  return (
    <div className="space-y-4">
      {messages.map((message, index) => (
        <ChatMessage key={message.id || index} message={message} />
      ))}
      <div ref={bottomRef} />
    </div>
  );
}
