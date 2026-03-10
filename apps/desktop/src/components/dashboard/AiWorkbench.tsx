/**
 * AI 工作台组件
 * 仪表盘页面的主要内容区域，全屏嵌入 AiPanel 组件。
 * 移除了原有的系统状态侧边栏，聊天区域占满可用空间。
 */

import React from 'react';
import { Bot } from 'lucide-react';

import { AiPanel } from '@/components/layout/AiPanel';

/**
 * AI 工作台 —— 无需外部 props，内部直接渲染全屏模式的 AiPanel。
 */
export const AiWorkbench: React.FC = () => {
  return (
    <div className="flex flex-col h-full">
      <section className="bg-white rounded-xl border border-gray-100 shadow-sm overflow-hidden flex flex-col flex-1">
        {/* 顶部标题栏 */}
        <header className="px-5 py-4 border-b border-gray-100 flex items-center gap-3 shrink-0">
          <div className="min-w-0">
            <h2 className="text-lg font-semibold text-gray-900 flex items-center gap-2">
              <Bot className="w-5 h-5 text-brand-600" />
              AI 工作台
            </h2>
            <p className="text-xs text-gray-500 mt-1">主对话区（支持角色切换与 / 快捷指令）</p>
          </div>
        </header>
        {/* 全屏 AiPanel 填满剩余空间 */}
        <div className="flex-1 min-h-0">
          <AiPanel mode="fullscreen" />
        </div>
      </section>
    </div>
  );
};
