/**
 * AI 助手面板组件
 * 右侧可展开的 AI 交互面板，提供教务助手功能
 */

import React, { useMemo, useState } from 'react';
import { Bot, Send, Sparkles, User, X } from 'lucide-react';
import { useToast } from '@/hooks/useToast';

interface AiPanelProps {
  isOpen: boolean;
  onClose: () => void;
}

export const AiPanel: React.FC<AiPanelProps> = ({ isOpen, onClose }) => {
  const { info } = useToast();
  const [inputText, setInputText] = useState('');
  const [messages, setMessages] = useState<Array<{ role: 'user' | 'assistant'; content: string }>>([]);

  const canSend = useMemo(() => inputText.trim().length > 0, [inputText]);

  const handleSend = () => {
    const trimmedText = inputText.trim();
    if (!trimmedText) return;

    setMessages((prev) => [
      ...prev,
      { role: 'user', content: trimmedText },
      {
        role: 'assistant',
        content: '已收到你的指令。当前右侧通用 AI 助手已开放输入，后续将接入完整对话能力。',
      },
    ]);
    setInputText('');
    info('已发送：当前通用 AI 助手为本地演示模式');
  };

  if (!isOpen) return null;

  return (
    <aside className="w-[360px] bg-white border-l border-gray-200 flex flex-col shadow-sm transition-all duration-300 ease-in-out shrink-0">
      <div className="h-14 flex items-center justify-between px-4 border-b border-gray-100 bg-brand-50/50">
        <div className="flex items-center gap-2 text-brand-700 font-medium">
          <Bot className="w-5 h-5" />
          <span>AI 助手</span>
        </div>
        <button
          onClick={onClose}
          className="p-1.5 rounded-md text-gray-400 hover:text-gray-600 hover:bg-gray-100 transition-colors"
          aria-label="关闭 AI 助手"
        >
          <X className="w-4 h-4" />
        </button>
      </div>
      
      <div className="flex-1 overflow-y-auto p-4">
        {messages.length === 0 ? (
          <div className="h-full flex flex-col items-center justify-center text-center text-gray-500">
            <div className="w-16 h-16 bg-brand-50 rounded-full flex items-center justify-center mb-4 text-brand-500">
              <Sparkles className="w-8 h-8" />
            </div>
            <h3 className="text-lg font-medium text-gray-900 mb-2">AI 助手就绪</h3>
            <p className="text-sm max-w-[220px]">我在这里协助您处理教务工作、分析成绩和生成评语。</p>
          </div>
        ) : (
          <div className="space-y-3">
            {messages.map((message, index) => (
              <div
                key={`${message.role}-${index}`}
                className={`flex gap-2 ${message.role === 'user' ? 'justify-end' : 'justify-start'}`}
              >
                {message.role === 'assistant' && (
                  <div className="w-7 h-7 rounded-full bg-brand-50 text-brand-600 flex items-center justify-center shrink-0">
                    <Bot className="w-4 h-4" />
                  </div>
                )}
                <div
                  className={`max-w-[260px] text-sm px-3 py-2 rounded-lg leading-6 ${
                    message.role === 'user'
                      ? 'bg-brand-600 text-white'
                      : 'bg-gray-100 text-gray-700 border border-gray-200'
                  }`}
                >
                  {message.content}
                </div>
                {message.role === 'user' && (
                  <div className="w-7 h-7 rounded-full bg-brand-600 text-white flex items-center justify-center shrink-0">
                    <User className="w-4 h-4" />
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
      
      <div className="p-4 border-t border-gray-100 bg-gray-50">
        <div className="relative">
          <input
            type="text"
            value={inputText}
            onChange={(event) => setInputText(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter') {
                event.preventDefault();
                handleSend();
              }
            }}
            placeholder="输入指令..."
            className="w-full pl-4 pr-10 py-2.5 bg-white border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-brand-500 focus:border-transparent transition-shadow"
          />
          <button
            type="button"
            onClick={handleSend}
            disabled={!canSend}
            className="absolute right-2 top-1/2 -translate-y-1/2 p-1.5 text-gray-400 hover:text-brand-600 transition-colors"
          >
            <Send className="w-4 h-4" />
          </button>
        </div>
      </div>
    </aside>
  );
};
