/**
 * 聊天输入框组件
 */

import React, { useState, useCallback, KeyboardEvent } from 'react';

export interface ChatInputProps {
  onSend: (message: string) => void;
  disabled?: boolean;
  placeholder?: string;
}

export function ChatInput({ onSend, disabled = false, placeholder = '输入消息...' }: ChatInputProps) {
  const [message, setMessage] = useState('');

  const handleSend = useCallback(() => {
    if (message.trim() && !disabled) {
      onSend(message.trim());
      setMessage('');
    }
  }, [message, disabled, onSend]);

  const handleKeyDown = useCallback((e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }, [handleSend]);

  return (
    <div className="flex gap-2">
      <textarea
        value={message}
        onChange={(e) => setMessage(e.target.value)}
        onKeyDown={handleKeyDown}
        disabled={disabled}
        placeholder={placeholder}
        rows={1}
        className="flex-1 resize-none rounded-lg border border-gray-300 px-4 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:bg-gray-100 disabled:text-gray-500"
        style={{ minHeight: '44px', maxHeight: '120px' }}
      />
      <button
        onClick={handleSend}
        disabled={disabled || !message.trim()}
        className="px-6 py-2 bg-blue-500 text-white rounded-lg font-medium disabled:bg-gray-300 disabled:cursor-not-allowed hover:bg-blue-600 transition-colors"
      >
        发送
      </button>
    </div>
  );
}
