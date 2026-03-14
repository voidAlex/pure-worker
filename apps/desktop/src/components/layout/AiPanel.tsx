/**
 * AI 助手面板组件
 * 支持两种渲染模式：
 * - sidebar：右侧可展开的侧边栏模式（非仪表盘页面使用）
 * - fullscreen：全屏嵌入模式，类似 ChatGPT 的聊天布局（仪表盘 AiWorkbench 使用）
 *
 * 功能包括：角色切换、斜杠快捷指令、消息发送与自动滚动、真实后端对话调用。
 */

import React, { useEffect, useMemo, useRef, useState } from 'react';
import { Bot, ChevronDown, Command, Loader2, Send, Sparkles, X } from 'lucide-react';

import { ChatMessage } from '@/components/chat/ChatMessage';
import { commands } from '@/services/commandClient';
import { isTauriRuntime } from '@/utils/runtime';

/** AI 角色类型 */
type AgentRole = 'homeroom' | 'grading' | 'communication' | 'ops';

/** 斜杠快捷指令定义 */
type SlashCommand = {
  key: string;
  title: string;
  description: string;
  execute: () => void;
};

/** 聊天消息结构 */
interface AiPanelMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  created_at: string;
  isStreaming?: boolean;
}

/**
 * 面板属性 —— 使用判别联合类型区分两种模式。
 * - sidebar 模式需要 isOpen 和 onClose
 * - fullscreen 模式仅接受可选 className
 */
type AiPanelProps =
  | { mode: 'sidebar'; isOpen: boolean; onClose: () => void }
  | { mode: 'fullscreen'; className?: string };

/** 角色中文名映射表 */
const AGENT_LABEL_MAP: Record<AgentRole, string> = {
  homeroom: '班主任助手',
  grading: '批改助手',
  communication: '沟通助手',
  ops: '教务助手',
};

/**
 * AI 助手面板主组件
 * 根据 mode 属性渲染不同的外层容器，内部聊天逻辑共用。
 */
export const AiPanel: React.FC<AiPanelProps> = (props) => {
  const [inputText, setInputText] = useState('');
  const [selectedAgent, setSelectedAgent] = useState<AgentRole>('homeroom');
  const [showAgentMenu, setShowAgentMenu] = useState(false);
  const [showSlashMenu, setShowSlashMenu] = useState(false);
  const [slashQuery, setSlashQuery] = useState('');
  const [messages, setMessages] = useState<AiPanelMessage[]>([]);
  const [isLoading, setIsLoading] = useState(false);

  /** 消息列表容器引用，用于自动滚动到底部 */
  const messagesEndRef = useRef<HTMLDivElement>(null);

  /** 消息变化时自动滚动到底部 */
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, isLoading]);

  /** 判断是否可以发送消息 */
  const canSend = useMemo(() => inputText.trim().length > 0 && !isLoading, [inputText, isLoading]);

  /** 是否为全屏模式 */
  const isFullscreen = props.mode === 'fullscreen';

  /** 可用的斜杠快捷指令列表 */
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

  /** 根据输入过滤斜杠命令 */
  const filteredCommands = useMemo(() => {
    const query = slashQuery.trim().toLowerCase();
    if (!query) {
      return availableCommands;
    }
    return availableCommands.filter((item) =>
      `${item.title} ${item.description}`.toLowerCase().includes(query),
    );
  }, [availableCommands, slashQuery]);

  /**
   * 发送消息处理函数
   * 添加用户消息后调用真实后端 chat_with_ai IPC 命令获取 AI 回复。
   * 非 Tauri 运行时显示友好提示。
   */
  const handleSend = async () => {
    const trimmedText = inputText.trim();
    if (!trimmedText || isLoading) return;

    // 立即添加用户消息并清空输入
    const userMessageId = `user-${Date.now()}`;
    setMessages((prev) => [
      ...prev,
      {
        id: userMessageId,
        role: 'user',
        content: trimmedText,
        created_at: new Date().toISOString(),
      },
    ]);
    setInputText('');
    setShowSlashMenu(false);
    setSlashQuery('');
    setIsLoading(true);

    try {
      // 非 Tauri 运行时直接返回提示
      if (!isTauriRuntime()) {
        setMessages((prev) => [
          ...prev,
          {
            id: `assistant-${Date.now()}`,
            role: 'assistant',
            content: '当前为 Web 预览环境，AI 对话需要桌面端支持。',
            created_at: new Date().toISOString(),
          },
        ]);
        return;
      }

      // 调用真实后端 AI 对话命令
      const result = await commands.chatWithAi({
        message: trimmedText,
        agent_role: selectedAgent,
        use_agentic_search: false,
      });

      if (result.status === 'error') {
        const errorMsg =
          typeof result.error === 'string' ? result.error : JSON.stringify(result.error);
        setMessages((prev) => [
          ...prev,
          {
            id: `error-${Date.now()}`,
            role: 'assistant',
            content: `抱歉，AI 响应失败：${errorMsg}`,
            created_at: new Date().toISOString(),
          },
        ]);
        return;
      }

      setMessages((prev) => [
        ...prev,
        {
          id: `assistant-${Date.now()}`,
          role: 'assistant',
          content: result.data.content,
          created_at: new Date().toISOString(),
        },
      ]);
    } catch (err: unknown) {
      const errorDetail = err instanceof Error ? err.message : String(err);
      setMessages((prev) => [
        ...prev,
        {
          id: `error-${Date.now()}`,
          role: 'assistant',
          content: `抱歉，AI 响应失败：${errorDetail}`,
          created_at: new Date().toISOString(),
        },
      ]);
    } finally {
      setIsLoading(false);
    }
  };

  // sidebar 模式下，未打开时不渲染
  if (props.mode === 'sidebar' && !props.isOpen) return null;

  /** 渲染顶部工具栏（角色切换 + 关闭按钮） */
  const renderHeader = () => (
    <div className="h-14 flex items-center justify-between px-4 border-b border-gray-100 bg-brand-50/50 gap-2 shrink-0">
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
          <span>{AGENT_LABEL_MAP[selectedAgent]}</span>
          <ChevronDown className="w-3.5 h-3.5" />
        </button>
        {showAgentMenu && (
          <div className="absolute right-0 mt-2 w-40 bg-white border border-gray-200 rounded-lg shadow-lg z-20 p-1">
            {(Object.keys(AGENT_LABEL_MAP) as AgentRole[]).map((role) => (
              <button
                key={role}
                type="button"
                onClick={() => {
                  setSelectedAgent(role);
                  setShowAgentMenu(false);
                  setMessages((prev) => [
                    ...prev,
                    {
                      id: `system-${Date.now()}`,
                      role: 'assistant',
                      content: `已切换角色：${AGENT_LABEL_MAP[role]}`,
                      created_at: new Date().toISOString(),
                    },
                  ]);
                }}
                className={`w-full text-left px-2.5 py-1.5 text-sm rounded-md transition-colors ${
                  selectedAgent === role
                    ? 'bg-brand-50 text-brand-700'
                    : 'text-gray-700 hover:bg-gray-50'
                }`}
              >
                {AGENT_LABEL_MAP[role]}
              </button>
            ))}
          </div>
        )}
      </div>
      {/* sidebar 模式显示关闭按钮 */}
      {props.mode === 'sidebar' && (
        <button
          onClick={props.onClose}
          className="p-1.5 rounded-md text-gray-400 hover:text-gray-600 hover:bg-gray-100 transition-colors"
          aria-label="关闭 AI 助手"
        >
          <X className="w-4 h-4" />
        </button>
      )}
    </div>
  );

  /** 渲染消息列表区域 */
  const renderMessages = () => (
    <div className="flex-1 overflow-y-auto p-4">
      {messages.length === 0 && !isLoading ? (
        <div className="h-full flex flex-col items-center justify-center text-center text-gray-500">
          <div className="w-16 h-16 bg-brand-50 rounded-full flex items-center justify-center mb-4 text-brand-500">
            <Sparkles className="w-8 h-8" />
          </div>
          <h3 className="text-lg font-medium text-gray-900 mb-2">AI 助手已就绪</h3>
          <p className="text-sm max-w-[280px]">我在这里协助您处理教务工作、分析成绩和生成评语。</p>
        </div>
      ) : (
        <div className={isFullscreen ? 'mx-auto max-w-3xl w-full' : ''}>
          <div className="space-y-3">
            {messages.map((message) => (
              <ChatMessage key={message.id} message={message} />
            ))}
            {/* AI 思考中指示器 */}
            {isLoading && (
              <div className="flex gap-2 justify-start">
                <div className="w-7 h-7 rounded-full bg-brand-50 text-brand-600 flex items-center justify-center shrink-0">
                  <Bot className="w-4 h-4" />
                </div>
                <div className="bg-gray-100 text-gray-500 border border-gray-200 text-sm px-3 py-2 rounded-lg flex items-center gap-2">
                  <Loader2 className="w-4 h-4 animate-spin" />
                  思考中...
                </div>
              </div>
            )}
            <div ref={messagesEndRef} />
          </div>
        </div>
      )}
    </div>
  );

  /** 渲染输入区域（含斜杠菜单） */
  const renderInput = () => (
    <div className="p-4 border-t border-gray-100 bg-gray-50 shrink-0">
      <div className={isFullscreen ? 'mx-auto max-w-3xl w-full' : ''}>
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
            disabled={isLoading}
            className="w-full pl-4 pr-10 py-2.5 bg-white border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-brand-500 focus:border-transparent transition-shadow disabled:opacity-50 disabled:cursor-not-allowed"
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
            className="absolute right-2 top-1/2 -translate-y-1/2 p-1.5 text-gray-400 hover:text-brand-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <Send className="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>
  );

  // 根据模式选择不同的外层容器
  if (isFullscreen) {
    return (
      <div
        className={`flex flex-col h-full w-full ${props.mode === 'fullscreen' ? (props.className ?? '') : ''}`}
      >
        {renderHeader()}
        {renderMessages()}
        {renderInput()}
      </div>
    );
  }

  return (
    <aside className="w-[360px] bg-white border-l border-gray-200 flex flex-col shadow-sm transition-all duration-300 ease-in-out shrink-0">
      {renderHeader()}
      {renderMessages()}
      {renderInput()}
    </aside>
  );
};
