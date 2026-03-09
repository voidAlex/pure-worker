/**
 * AI 助手面板组件
 * 右侧可展开的 AI 交互面板，提供教务助手功能
 */

import React, { useMemo, useState } from 'react';
import { Bot, ChevronDown, Command, Send, Sparkles, User, X } from 'lucide-react';

type AgentRole = 'homeroom' | 'grading' | 'communication' | 'ops';

type SlashCommand = {
  key: string;
  title: string;
  description: string;
  execute: () => void;
};

interface AiPanelProps {
  isOpen: boolean;
  onClose: () => void;
}

export const AiPanel: React.FC<AiPanelProps> = ({ isOpen, onClose }) => {
  const [inputText, setInputText] = useState('');
  const [selectedAgent, setSelectedAgent] = useState<AgentRole>('homeroom');
  const [showAgentMenu, setShowAgentMenu] = useState(false);
  const [showSlashMenu, setShowSlashMenu] = useState(false);
  const [slashQuery, setSlashQuery] = useState('');
  const [messages, setMessages] = useState<Array<{ role: 'user' | 'assistant'; content: string }>>([]);

  const agentLabelMap: Record<AgentRole, string> = {
    homeroom: '班主任助手',
    grading: '批改助手',
    communication: '沟通助手',
    ops: '教务助手',
  };

  const canSend = useMemo(() => inputText.trim().length > 0, [inputText]);

  const availableCommands = useMemo<SlashCommand[]>(
    () => [
      {
        key: '/agent',
        title: '/agent',
        description: '切换 AI 角色',
        execute: () => {
          setShowAgentMenu(true);
          setInputText('');
          setShowSlashMenu(false);
        },
      },
      {
        key: '/clear',
        title: '/clear',
        description: '清空聊天记录',
        execute: () => {
          setMessages([]);
          setInputText('');
          setShowSlashMenu(false);
        },
      },
      {
        key: '/new',
        title: '/new',
        description: '开启新会话',
        execute: () => {
          setMessages([]);
          setInputText('');
          setShowSlashMenu(false);
        },
      },
    ],
    [],
  );

  const filteredCommands = useMemo(() => {
    const query = slashQuery.trim().toLowerCase();
    if (!query) {
      return availableCommands;
    }
    return availableCommands.filter((item) =>
      `${item.title} ${item.description}`.toLowerCase().includes(query),
    );
  }, [availableCommands, slashQuery]);

  const handleSend = () => {
    const trimmedText = inputText.trim();
    if (!trimmedText) return;

    setMessages((prev) => [
      ...prev,
      { role: 'user', content: trimmedText },
      {
        role: 'assistant',
        content: `已收到你的指令。当前由${agentLabelMap[selectedAgent]}处理，你也可以输入 / 呼出快捷指令。`,
      },
    ]);
    setInputText('');
    setShowSlashMenu(false);
    setSlashQuery('');
  };

  if (!isOpen) return null;

  return (
    <aside className="w-[360px] bg-white border-l border-gray-200 flex flex-col shadow-sm transition-all duration-300 ease-in-out shrink-0">
      <div className="h-14 flex items-center justify-between px-4 border-b border-gray-100 bg-brand-50/50 gap-2">
        <div className="flex items-center gap-2 text-brand-700 font-medium min-w-0">
          <Bot className="w-5 h-5 shrink-0" />
          <span className="truncate">AI 助手</span>
        </div>
        <div className="relative">
          <button
            type="button"
            onClick={() => setShowAgentMenu((prev) => !prev)}
            className="inline-flex items-center gap-1.5 px-2.5 py-1 text-xs rounded-md bg-white border border-brand-100 text-brand-700 hover:bg-brand-50 transition-colors"
          >
            <span>{agentLabelMap[selectedAgent]}</span>
            <ChevronDown className="w-3.5 h-3.5" />
          </button>
          {showAgentMenu && (
            <div className="absolute right-0 mt-2 w-40 bg-white border border-gray-200 rounded-lg shadow-lg z-20 p-1">
              {(Object.keys(agentLabelMap) as AgentRole[]).map((role) => (
                <button
                  key={role}
                  type="button"
                  onClick={() => {
                    setSelectedAgent(role);
                    setShowAgentMenu(false);
                    setMessages((prev) => [
                      ...prev,
                      { role: 'assistant', content: `已切换角色：${agentLabelMap[role]}` },
                    ]);
                  }}
                  className={`w-full text-left px-2.5 py-1.5 text-sm rounded-md transition-colors ${
                    selectedAgent === role
                      ? 'bg-brand-50 text-brand-700'
                      : 'text-gray-700 hover:bg-gray-50'
                  }`}
                >
                  {agentLabelMap[role]}
                </button>
              ))}
            </div>
          )}
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
            <h3 className="text-lg font-medium text-gray-900 mb-2">AI 助手已就绪</h3>
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
            onChange={(event) => {
              const nextValue = event.target.value;
              setInputText(nextValue);
              if (nextValue.startsWith('/')) {
                setShowSlashMenu(true);
                setSlashQuery(nextValue.slice(1));
              } else {
                setShowSlashMenu(false);
                setSlashQuery('');
              }
            }}
            onKeyDown={(event) => {
              if (event.key === 'Enter' && showSlashMenu && filteredCommands.length > 0) {
                event.preventDefault();
                filteredCommands[0].execute();
                return;
              }

              if (event.key === 'Enter' && !event.shiftKey) {
                event.preventDefault();
                handleSend();
              }
            }}
            placeholder="输入消息，或键入 / 呼出快捷指令"
            className="w-full pl-4 pr-10 py-2.5 bg-white border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-brand-500 focus:border-transparent transition-shadow"
          />
          {showSlashMenu && (
            <div className="absolute left-0 right-0 bottom-12 bg-white border border-gray-200 rounded-lg shadow-lg z-20 p-1">
              {filteredCommands.length > 0 ? (
                filteredCommands.map((item) => (
                  <button
                    key={item.key}
                    type="button"
                    onClick={item.execute}
                    className="w-full text-left px-2.5 py-2 rounded-md hover:bg-gray-50 transition-colors"
                  >
                    <div className="flex items-center gap-2 text-sm font-medium text-gray-800">
                      <Command className="w-3.5 h-3.5 text-gray-500" />
                      {item.title}
                    </div>
                    <div className="text-xs text-gray-500 mt-0.5">{item.description}</div>
                  </button>
                ))
              ) : (
                <div className="px-2.5 py-2 text-xs text-gray-500">未找到匹配命令</div>
              )}
            </div>
          )}
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
