import React from 'react';
import { Bot, CheckCircle2, Command, Sparkles } from 'lucide-react';

import { AiPanel } from '@/components/layout/AiPanel';

interface AiWorkbenchProps {
  healthText: string;
  taskCount: number;
}

export const AiWorkbench: React.FC<AiWorkbenchProps> = ({ healthText, taskCount }) => {
  return (
    <div className="grid grid-cols-1 xl:grid-cols-[minmax(0,1fr)_340px] gap-4 h-full min-h-[640px]">
      <section className="bg-white rounded-xl border border-gray-100 shadow-sm overflow-hidden flex flex-col min-h-[640px]">
        <header className="px-5 py-4 border-b border-gray-100 flex items-center justify-between gap-3">
          <div className="min-w-0">
            <h2 className="text-lg font-semibold text-gray-900 flex items-center gap-2">
              <Bot className="w-5 h-5 text-brand-600" />
              AI 工作台
            </h2>
            <p className="text-xs text-gray-500 mt-1">主对话区（支持角色切换与 / 快捷指令）</p>
          </div>
          <div className="inline-flex items-center gap-2 px-2.5 py-1.5 rounded-lg bg-brand-50 text-brand-700 text-xs font-medium">
            <Sparkles className="w-3.5 h-3.5" />
            AI 主导
          </div>
        </header>
        <div className="flex-1 min-h-0">
          <AiPanel isOpen onClose={() => {}} />
        </div>
      </section>

      <aside className="space-y-4">
        <section className="bg-white rounded-xl border border-gray-100 shadow-sm p-4">
          <h3 className="text-sm font-semibold text-gray-900 mb-3">系统状态</h3>
          <div className="space-y-2 text-sm text-gray-600">
            <div className="flex items-center justify-between">
              <span>后端健康</span>
              <span className="inline-flex items-center gap-1 text-green-600 font-medium">
                <CheckCircle2 className="w-4 h-4" />
                {healthText}
              </span>
            </div>
            <div className="flex items-center justify-between">
              <span>待办任务</span>
              <span className="font-medium text-gray-900">{taskCount} 项</span>
            </div>
          </div>
        </section>

        <section className="bg-white rounded-xl border border-gray-100 shadow-sm p-4">
          <h3 className="text-sm font-semibold text-gray-900 mb-3">快捷提示</h3>
          <ul className="space-y-2 text-xs text-gray-600">
            <li className="flex items-start gap-2">
              <Command className="w-3.5 h-3.5 mt-0.5 text-gray-500" />
              输入 <span className="font-mono text-gray-800">/agent</span> 快速切换 AI 角色
            </li>
            <li className="flex items-start gap-2">
              <Command className="w-3.5 h-3.5 mt-0.5 text-gray-500" />
              输入 <span className="font-mono text-gray-800">/new</span> 新建会话
            </li>
            <li className="flex items-start gap-2">
              <Command className="w-3.5 h-3.5 mt-0.5 text-gray-500" />
              输入 <span className="font-mono text-gray-800">/clear</span> 清空当前会话
            </li>
          </ul>
        </section>
      </aside>
    </div>
  );
};
