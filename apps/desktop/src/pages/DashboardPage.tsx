/**
 * 工作台页面组件
 * 仪表盘首页，内嵌 AI 工作台，占满可用高度。
 */

import React from 'react';

import { ChatPanel } from '@/components/chat/ChatPanel';

/**
 * 仪表盘页面 —— 渲染顶部标题 + AI 工作台。
 */
export const DashboardPage: React.FC = () => {
  return (
    <div className="h-full flex flex-col">
      {/* 页面标题区 */}
      <header className="flex items-center justify-between shrink-0">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">工作台</h1>
          <p className="text-sm text-gray-500 mt-1">欢迎使用 PureWorker 教务助手</p>
        </div>
      </header>

      {/* AI 聊天面板，填满剩余空间 */}
      <div className="flex-1 min-h-0 mt-6">
        <ChatPanel agentRole="homeroom" />
      </div>
    </div>
  );
};
