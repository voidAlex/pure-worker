/**
 * 单条聊天消息组件
 */

import React from 'react';
import ReactMarkdown, { Components } from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { ChatMessage as ChatMessageType } from '@/hooks/useChatStream';

export interface ChatMessageProps {
  message: ChatMessageType;
}

export function ChatMessage({ message }: ChatMessageProps) {
  const isUser = message.role === 'user';

  const components: Components = {
    code({ className, children, ...props }) {
      const match = /language-(\w+)/.exec(className || '');
      const isInline = !match;

      return isInline ? (
        <code
          className={`${isUser ? 'bg-blue-600' : 'bg-gray-200'} px-1 py-0.5 rounded text-sm ${className || ''}`}
          {...props}
        >
          {children}
        </code>
      ) : (
        <code className={className} {...props}>
          {children}
        </code>
      );
    },
    pre({ children, ...props }) {
      return (
        <pre
          className="bg-gray-800 text-gray-100 p-3 rounded-lg overflow-x-auto text-sm my-2"
          {...props}
        >
          {children}
        </pre>
      );
    },
    p({ children }) {
      return <p className="mb-2 last:mb-0">{children}</p>;
    },
    a({ href, children }) {
      return (
        <a
          href={href}
          className="underline decoration-1 underline-offset-2 hover:opacity-80"
          target="_blank"
          rel="noreferrer"
        >
          {children}
        </a>
      );
    },
  };

  return (
    <div className={`flex ${isUser ? 'justify-end' : 'justify-start'}`}>
      <div
        className={`max-w-[80%] rounded-2xl px-4 py-3 ${
          isUser ? 'bg-blue-500 text-white' : 'bg-gray-100 text-gray-800'
        }`}
      >
        <div className={`text-xs mb-1 ${isUser ? 'text-blue-200' : 'text-gray-500'}`}>
          {isUser ? '我' : 'AI 助手'}
        </div>
        <div
          className={`prose prose-sm max-w-none ${isUser ? 'prose-invert' : 'dark:prose-invert'} leading-relaxed`}
        >
          <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
            {message.content || (message.isStreaming ? '' : '...')}
          </ReactMarkdown>
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
