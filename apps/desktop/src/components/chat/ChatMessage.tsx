/**
 * 单条聊天消息组件
 */

import React from 'react';
import ReactMarkdown, { Components } from 'react-markdown';
import remarkGfm from 'remark-gfm';
import type { ChatMessageItem as ChatMessageType } from './types';

export interface ChatMessageProps {
  message: ChatMessageType;
}

const STAGE_META: Record<string, { label: string; badgeClass: string }> = {
  searching: {
    label: '检索中',
    badgeClass: 'bg-sky-100 text-sky-700 border-sky-200',
  },
  reasoning: {
    label: '推理中',
    badgeClass: 'bg-amber-100 text-amber-800 border-amber-200',
  },
  tool_calling: {
    label: '工具调用',
    badgeClass: 'bg-violet-100 text-violet-800 border-violet-200',
  },
  generating: {
    label: '生成中',
    badgeClass: 'bg-emerald-100 text-emerald-800 border-emerald-200',
  },
  search_failed: {
    label: '检索失败',
    badgeClass: 'bg-rose-100 text-rose-800 border-rose-200',
  },
  complete: {
    label: '已完成',
    badgeClass: 'bg-slate-100 text-slate-700 border-slate-200',
  },
};

function formatToolPayload(payload: unknown): string {
  if (payload === undefined || payload === null) {
    return '无';
  }

  if (typeof payload === 'string') {
    return payload;
  }

  try {
    return JSON.stringify(payload, null, 2);
  } catch {
    return String(payload);
  }
}

export function ChatMessage({ message }: ChatMessageProps) {
  const isUser = message.role === 'user';
  const stageMeta = message.thinkingTrace
    ? STAGE_META[message.thinkingTrace.stage] || {
        label: message.thinkingTrace.stage,
        badgeClass: 'bg-slate-100 text-slate-700 border-slate-200',
      }
    : null;

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
      {isUser ? (
        <div className="max-w-[80%] rounded-2xl px-4 py-3 bg-blue-500 text-white">
          <div className="text-xs mb-1 text-blue-200">我</div>
          <div className="prose prose-sm prose-invert max-w-none leading-relaxed">
            <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
              {message.content || (message.isStreaming ? '' : '...')}
            </ReactMarkdown>
          </div>
        </div>
      ) : (
        <div className="max-w-[90%] w-full space-y-2">
          <div className="text-xs text-gray-500 px-1">AI 助手</div>

          {message.thinkingTrace && stageMeta && (
            <section className="rounded-xl border border-sky-100 bg-sky-50 px-4 py-3">
              <div className="flex items-center gap-2 mb-2">
                <span
                  className={`inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-medium ${stageMeta.badgeClass}`}
                >
                  {stageMeta.label}
                </span>
              </div>
              <p className="text-sm text-slate-700 leading-relaxed">
                {message.thinkingTrace.description || '正在处理你的请求...'}
              </p>
            </section>
          )}

          {message.thinkingTrace?.reasoning && (
            <section className="rounded-xl border border-amber-100 bg-amber-50 px-4 py-3">
              <div className="text-xs font-medium text-amber-800 mb-2">思考摘要</div>
              <p className="text-sm text-slate-700 leading-relaxed">
                {message.thinkingTrace.reasoning}
              </p>
            </section>
          )}

          {message.thinkingTrace?.toolCalls.map((toolCall, index) => (
            <section
              key={`${toolCall.toolName}-${index}`}
              className="rounded-xl border border-violet-100 bg-violet-50 px-4 py-3"
            >
              <div className="text-xs font-medium text-violet-800 mb-2">
                工具调用 · {toolCall.toolName}
              </div>
              <div className="space-y-2 text-xs">
                <div>
                  <div className="text-slate-500 mb-1">输入参数</div>
                  <pre className="bg-white border border-violet-100 text-slate-700 p-2 rounded-lg overflow-x-auto">
                    {formatToolPayload(toolCall.input)}
                  </pre>
                </div>
                {toolCall.output !== undefined && (
                  <div>
                    <div className="text-slate-500 mb-1">执行结果</div>
                    <pre className="bg-white border border-violet-100 text-slate-700 p-2 rounded-lg overflow-x-auto">
                      {toolCall.output}
                    </pre>
                  </div>
                )}
              </div>
              {toolCall.success !== undefined && (
                <div className="mt-2 text-xs text-slate-500">
                  状态：{toolCall.success ? '成功' : '失败'}
                </div>
              )}
            </section>
          ))}

          <section className="rounded-2xl border border-slate-200 bg-white px-4 py-3 shadow-sm">
            <div className="text-xs mb-2 text-gray-500">正文</div>
            <div className="prose prose-sm max-w-none leading-relaxed text-slate-800">
              <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
                {message.content || (message.isStreaming ? '' : '...')}
              </ReactMarkdown>
            </div>
            {message.tool_name && (
              <div className="text-xs mt-2 text-gray-500">使用了工具: {message.tool_name}</div>
            )}
          </section>
        </div>
      )}
    </div>
  );
}
